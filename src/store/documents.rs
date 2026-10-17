use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult, validate_body_size};
use crate::store::projects::access_in;
use crate::store::specs::spec_by_number;
use crate::store::{Store, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionMeta {
  pub id: i64,
  pub author: String,
  pub created_at: String,
  pub message: String,
  pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
  pub body: String,
  pub revision: RevisionMeta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constitution {
  pub body: String,
  pub version: String,
  pub revision: RevisionMeta,
}

fn reject_constitution_kind(kind: DocumentKind) -> StoreResult<()> {
  if kind == DocumentKind::Constitution {
    return Err(StoreError::Validation(
      "the constitution is read and written with the constitution operations, not as a spec document".into(),
    ));
  }
  Ok(())
}

pub fn document_id(
  conn: &Connection,
  project_id: i64,
  spec_id: Option<i64>,
  kind: DocumentKind,
) -> StoreResult<Option<i64>> {
  Ok(
    conn
      .query_row(
        "SELECT id FROM documents WHERE project_id = ?1 AND spec_id IS ?2 AND kind = ?3",
        params![project_id, spec_id, kind.as_str()],
        |r| r.get(0),
      )
      .optional()?,
  )
}

pub fn current_revision(
  conn: &Connection,
  document_id: i64,
) -> StoreResult<Option<(String, RevisionMeta)>> {
  Ok(
    conn
      .query_row(
        "SELECT r.body, r.id, u.handle, r.created_at, r.message, r.version
         FROM documents d
         JOIN revisions r ON r.id = d.current_revision_id
         JOIN users u ON u.id = r.author_id
         WHERE d.id = ?1",
        [document_id],
        |r| {
          Ok((
            r.get(0)?,
            RevisionMeta {
              id: r.get(1)?,
              author: r.get(2)?,
              created_at: r.get(3)?,
              message: r.get(4)?,
              version: r.get(5)?,
            },
          ))
        },
      )
      .optional()?,
  )
}

fn current_revision_id(conn: &Connection, document_id: i64) -> StoreResult<Option<i64>> {
  Ok(conn.query_row(
    "SELECT current_revision_id FROM documents WHERE id = ?1",
    [document_id],
    |r| r.get(0),
  )?)
}

fn check_expected_revision(
  current: Option<i64>,
  expected: Option<i64>,
  what: &str,
) -> StoreResult<()> {
  match (current, expected) {
    (Some(current), Some(expected)) if current == expected => Ok(()),
    (Some(current), _) => Err(StoreError::Conflict {
      current_revision: current,
    }),
    (None, Some(_)) => Err(StoreError::Validation(format!(
      "{what} does not exist yet; omit expected_revision when creating it"
    ))),
    (None, None) => Ok(()),
  }
}

fn ensure_document(
  conn: &Connection,
  project_id: i64,
  spec_id: Option<i64>,
  kind: DocumentKind,
) -> StoreResult<i64> {
  if let Some(id) = document_id(conn, project_id, spec_id, kind)? {
    return Ok(id);
  }
  conn.execute(
    "INSERT INTO documents (project_id, spec_id, kind) VALUES (?1, ?2, ?3)",
    params![project_id, spec_id, kind.as_str()],
  )?;
  Ok(conn.last_insert_rowid())
}

pub(crate) fn append_revision(
  conn: &Connection,
  document_id: i64,
  body: &str,
  version: Option<&str>,
  author_id: i64,
  message: &str,
) -> StoreResult<i64> {
  conn.execute(
    "INSERT INTO revisions (document_id, body, version, author_id, message) VALUES (?1, ?2, ?3, ?4, ?5)",
    params![document_id, body, version, author_id, message],
  )?;
  let revision_id = conn.last_insert_rowid();
  conn.execute(
    "UPDATE documents SET current_revision_id = ?1 WHERE id = ?2",
    params![revision_id, document_id],
  )?;
  Ok(revision_id)
}

impl Store {
  pub fn get_document(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
    kind: DocumentKind,
  ) -> StoreResult<Document> {
    reject_constitution_kind(kind)?;
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let spec = spec_by_number(conn, access.project_id, project, number)?;
      let missing = || StoreError::NotFound(format!("{kind} document of spec {number:03}"));
      let doc_id =
        document_id(conn, access.project_id, Some(spec.id), kind)?.ok_or_else(missing)?;
      let (body, revision) = current_revision(conn, doc_id)?.ok_or_else(missing)?;
      Ok(Document { body, revision })
    })
  }

  pub fn get_constitution(&self, caller: &Caller, project: &str) -> StoreResult<Constitution> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let missing = || StoreError::NotFound(format!("constitution of project {project:?}"));
      let doc_id = document_id(conn, access.project_id, None, DocumentKind::Constitution)?
        .ok_or_else(missing)?;
      let (body, revision) = current_revision(conn, doc_id)?.ok_or_else(missing)?;
      let version = revision.version.clone().unwrap_or_default();
      Ok(Constitution {
        body,
        version,
        revision,
      })
    })
  }

  #[allow(clippy::too_many_arguments)]
  pub fn put_document(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
    kind: DocumentKind,
    body: &str,
    message: &str,
    expected_revision: Option<i64>,
  ) -> StoreResult<i64> {
    reject_constitution_kind(kind)?;
    validate_body_size(body)?;
    self.with_tx(|tx| {
      let access = access_in(tx, caller, project)?;
      if !access.can_write() {
        return Err(StoreError::Forbidden);
      }
      let spec = spec_by_number(tx, access.project_id, project, number)?;
      if spec.status == SpecStatus::Archived {
        return Err(StoreError::Forbidden);
      }
      let existing = document_id(tx, access.project_id, Some(spec.id), kind)?;
      let current = match existing {
        Some(id) => current_revision_id(tx, id)?,
        None => None,
      };
      check_expected_revision(
        current,
        expected_revision,
        &format!("{kind} document of spec {number:03}"),
      )?;
      let doc_id = ensure_document(tx, access.project_id, Some(spec.id), kind)?;
      let revision_id = append_revision(tx, doc_id, body, None, caller.user_id, message)?;
      audit(
        tx,
        caller.user_id,
        Some(access.project_id),
        "document.put",
        &format!("{number:03}/{kind}@{revision_id}"),
      )?;
      Ok(revision_id)
    })
  }

  pub fn put_constitution(
    &self,
    caller: &Caller,
    project: &str,
    body: &str,
    version: &str,
    message: &str,
    expected_revision: Option<i64>,
  ) -> StoreResult<i64> {
    validate_body_size(body)?;
    self.with_tx(|tx| {
      let access = access_in(tx, caller, project)?;
      if !access.is_owner() {
        return Err(StoreError::Forbidden);
      }
      let existing = document_id(tx, access.project_id, None, DocumentKind::Constitution)?;
      let current = match existing {
        Some(id) => current_revision(tx, id)?.map(|(_, meta)| meta),
        None => None,
      };
      if let Some(meta) = &current
        && meta.version.as_deref() == Some(version)
      {
        return Err(StoreError::Conflict {
          current_revision: meta.id,
        });
      }
      check_expected_revision(
        current.as_ref().map(|m| m.id),
        expected_revision,
        &format!("constitution of project {project:?}"),
      )?;
      let doc_id = ensure_document(tx, access.project_id, None, DocumentKind::Constitution)?;
      let revision_id = append_revision(tx, doc_id, body, Some(version), caller.user_id, message)?;
      audit(
        tx,
        caller.user_id,
        Some(access.project_id),
        "constitution.put",
        &format!("v{version}@{revision_id}"),
      )?;
      Ok(revision_id)
    })
  }
}
