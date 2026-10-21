use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult, validate_body_size};
use crate::schema::{documents, revisions, specs, users};
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
  conn: &mut SqliteConnection,
  project_id: i64,
  spec_id: Option<i64>,
  kind: DocumentKind,
) -> StoreResult<Option<i64>> {
  Ok(
    documents::table
      .filter(documents::project_id.eq(project_id))
      .filter(documents::spec_id.is(spec_id))
      .filter(documents::kind.eq(kind.as_str()))
      .select(documents::id)
      .first(conn)
      .optional()?,
  )
}

type RevisionMetaRow = (i64, String, String, String, Option<String>);

fn revision_meta((id, author, created_at, message, version): RevisionMetaRow) -> RevisionMeta {
  RevisionMeta {
    id,
    author,
    created_at,
    message,
    version,
  }
}

pub fn current_revision(
  conn: &mut SqliteConnection,
  document_id: i64,
) -> StoreResult<Option<(String, RevisionMeta)>> {
  let row: Option<(String, RevisionMetaRow)> = documents::table
    .inner_join(revisions::table.on(documents::current_revision_id.eq(revisions::id.nullable())))
    .inner_join(users::table.on(users::id.eq(revisions::author_id)))
    .filter(documents::id.eq(document_id))
    .select((
      revisions::body,
      (
        revisions::id,
        users::handle,
        revisions::created_at,
        revisions::message,
        revisions::version,
      ),
    ))
    .first(conn)
    .optional()?;
  Ok(row.map(|(body, meta)| (body, revision_meta(meta))))
}

fn current_revision_id(conn: &mut SqliteConnection, document_id: i64) -> StoreResult<Option<i64>> {
  Ok(
    documents::table
      .filter(documents::id.eq(document_id))
      .select(documents::current_revision_id)
      .first(conn)?,
  )
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

pub(crate) fn ensure_document(
  conn: &mut SqliteConnection,
  project_id: i64,
  spec_id: Option<i64>,
  kind: DocumentKind,
) -> StoreResult<i64> {
  if let Some(id) = document_id(conn, project_id, spec_id, kind)? {
    return Ok(id);
  }
  Ok(
    diesel::insert_into(documents::table)
      .values((
        documents::project_id.eq(project_id),
        documents::spec_id.eq(spec_id),
        documents::kind.eq(kind.as_str()),
      ))
      .returning(documents::id)
      .get_result(conn)?,
  )
}

pub(crate) fn append_revision(
  conn: &mut SqliteConnection,
  document_id: i64,
  body: &str,
  version: Option<&str>,
  author_id: i64,
  message: &str,
) -> StoreResult<i64> {
  let revision_id: i64 = diesel::insert_into(revisions::table)
    .values((
      revisions::document_id.eq(document_id),
      revisions::body.eq(body),
      revisions::version.eq(version),
      revisions::author_id.eq(author_id),
      revisions::message.eq(message),
    ))
    .returning(revisions::id)
    .get_result(conn)?;
  diesel::update(documents::table.filter(documents::id.eq(document_id)))
    .set(documents::current_revision_id.eq(revision_id))
    .execute(conn)?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionBody {
  pub body: String,
  pub meta: RevisionMeta,
  pub kind: DocumentKind,
  pub number: Option<i64>,
}

impl Store {
  pub fn list_revisions(
    &self,
    caller: &Caller,
    project: &str,
    number: Option<i64>,
    kind: DocumentKind,
  ) -> StoreResult<Vec<RevisionMeta>> {
    match (number, kind) {
      (Some(_), DocumentKind::Constitution) => {
        return Err(StoreError::Validation(
          "the constitution belongs to the project; omit the spec number".into(),
        ));
      }
      (None, k) if k != DocumentKind::Constitution => {
        return Err(StoreError::Validation(format!(
          "a {k} document belongs to a spec; pass the spec number"
        )));
      }
      _ => {}
    }
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let doc_id = match number {
        None => document_id(conn, access.project_id, None, DocumentKind::Constitution)?
          .ok_or_else(|| StoreError::NotFound(format!("constitution of project {project:?}")))?,
        Some(number) => {
          let spec = spec_by_number(conn, access.project_id, project, number)?;
          document_id(conn, access.project_id, Some(spec.id), kind)?
            .ok_or_else(|| StoreError::NotFound(format!("{kind} document of spec {number:03}")))?
        }
      };
      let rows: Vec<RevisionMetaRow> = revisions::table
        .inner_join(users::table)
        .filter(revisions::document_id.eq(doc_id))
        .select((
          revisions::id,
          users::handle,
          revisions::created_at,
          revisions::message,
          revisions::version,
        ))
        .order(revisions::id.desc())
        .load(conn)?;
      Ok(rows.into_iter().map(revision_meta).collect())
    })
  }

  pub fn get_revision(
    &self,
    caller: &Caller,
    project: &str,
    revision_id: i64,
  ) -> StoreResult<RevisionBody> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let row: Option<(RevisionMetaRow, String, String, Option<i64>)> = revisions::table
        .inner_join(documents::table)
        .inner_join(users::table)
        .left_join(specs::table.on(documents::spec_id.eq(specs::id.nullable())))
        .filter(revisions::id.eq(revision_id))
        .filter(documents::project_id.eq(access.project_id))
        .select((
          (
            revisions::id,
            users::handle,
            revisions::created_at,
            revisions::message,
            revisions::version,
          ),
          revisions::body,
          documents::kind,
          specs::number.nullable(),
        ))
        .first(conn)
        .optional()?;
      let (meta, body, kind, number) =
        row.ok_or_else(|| StoreError::NotFound(format!("revision {revision_id}")))?;
      Ok(RevisionBody {
        body,
        meta: revision_meta(meta),
        kind: kind.parse()?,
        number,
      })
    })
  }
}
