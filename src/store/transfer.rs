use rusqlite::{Connection, params};

use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult};
use crate::store::documents::{append_revision, document_id, ensure_document};
use crate::store::projects::access_in;
use crate::store::specs::branch_name;
use crate::store::{Store, audit};
use crate::tree::{ProjectTree, TreeSpec, status_word, version_line};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportOptions {
  pub constitution_version: Option<String>,
  pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSpec {
  pub branch: String,
  pub status: SpecStatus,
  pub kinds: Vec<DocumentKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
  pub constitution_version: Option<String>,
  pub specs: Vec<ImportedSpec>,
  pub status_fallbacks: Vec<String>,
  pub documents: usize,
}

fn spec_status(spec: &TreeSpec) -> Option<SpecStatus> {
  spec
    .documents
    .iter()
    .find(|(kind, _)| *kind == DocumentKind::Spec)
    .and_then(|(_, file)| status_word(&file.body))
    .and_then(|word| word.parse().ok())
}

fn existing_spec_problems(
  conn: &Connection,
  project_id: i64,
  project: &str,
  spec: &TreeSpec,
) -> StoreResult<Vec<String>> {
  let mut problems = Vec::new();
  let number_taken: i64 = conn.query_row(
    "SELECT count(*) FROM specs WHERE project_id = ?1 AND number = ?2",
    params![project_id, spec.number],
    |r| r.get(0),
  )?;
  if number_taken > 0 {
    problems.push(format!(
      "specs/{}: spec {:03} already exists in project {project:?}",
      spec.folder, spec.number
    ));
  }
  let slug_taken: i64 = conn.query_row(
    "SELECT count(*) FROM specs WHERE project_id = ?1 AND slug = ?2",
    params![project_id, spec.slug],
    |r| r.get(0),
  )?;
  if slug_taken > 0 {
    problems.push(format!(
      "specs/{}: slug {:?} already exists in project {project:?}",
      spec.folder, spec.slug
    ));
  }
  Ok(problems)
}

impl Store {
  pub fn import_tree(
    &self,
    caller: &Caller,
    project: &str,
    tree: &ProjectTree,
    options: &ImportOptions,
  ) -> StoreResult<ImportReport> {
    self.with_tx(|tx| {
      let access = access_in(tx, caller, project)?;
      if !access.is_owner() {
        return Err(StoreError::Forbidden);
      }
      let mut problems = tree.problems.clone();
      let mut constitution_version = None;
      if let Some(file) = &tree.constitution {
        if document_id(tx, access.project_id, None, DocumentKind::Constitution)?.is_some() {
          problems.push(format!(
            "{}: project {project:?} already has a constitution",
            file.relative_path
          ));
        }
        constitution_version = options
          .constitution_version
          .clone()
          .or_else(|| version_line(&file.body).map(str::to_string));
        if constitution_version.is_none() {
          problems.push(format!(
            "{}: no line starts with \"Version \"; pass the constitution version explicitly",
            file.relative_path
          ));
        }
      }
      for spec in &tree.specs {
        problems.extend(existing_spec_problems(
          tx,
          access.project_id,
          project,
          spec,
        )?);
      }
      if !problems.is_empty() {
        return Err(StoreError::Validation(problems.join("\n")));
      }

      let mut report = ImportReport {
        constitution_version: constitution_version.clone(),
        documents: usize::from(tree.constitution.is_some()),
        ..ImportReport::default()
      };
      let mut statuses = Vec::new();
      for spec in &tree.specs {
        let branch = branch_name(spec.number, &spec.slug);
        let status = spec_status(spec).unwrap_or_else(|| {
          report.status_fallbacks.push(branch.clone());
          SpecStatus::Draft
        });
        statuses.push(status);
        report.documents += spec.documents.len();
        report.specs.push(ImportedSpec {
          branch,
          status,
          kinds: spec.documents.iter().map(|(kind, _)| *kind).collect(),
        });
      }
      if options.dry_run {
        return Ok(report);
      }

      if let (Some(file), Some(version)) = (&tree.constitution, &constitution_version) {
        let doc_id = ensure_document(tx, access.project_id, None, DocumentKind::Constitution)?;
        let revision_id = append_revision(
          tx,
          doc_id,
          &file.body,
          Some(version),
          caller.user_id,
          &format!("import {}", file.relative_path),
        )?;
        audit(
          tx,
          caller.user_id,
          Some(access.project_id),
          "constitution.put",
          &format!("v{version}@{revision_id}"),
        )?;
      }
      for (spec, status) in tree.specs.iter().zip(statuses) {
        let branch = branch_name(spec.number, &spec.slug);
        tx.execute(
          "INSERT INTO specs (project_id, number, slug, status, branch) VALUES (?1, ?2, ?3, ?4, ?5)",
          params![access.project_id, spec.number, spec.slug, status.as_str(), branch],
        )?;
        let spec_id = tx.last_insert_rowid();
        audit(
          tx,
          caller.user_id,
          Some(access.project_id),
          "spec.import",
          &branch,
        )?;
        for (kind, file) in &spec.documents {
          let doc_id = ensure_document(tx, access.project_id, Some(spec_id), *kind)?;
          let revision_id = append_revision(
            tx,
            doc_id,
            &file.body,
            None,
            caller.user_id,
            &format!("import {}", file.relative_path),
          )?;
          audit(
            tx,
            caller.user_id,
            Some(access.project_id),
            "document.put",
            &format!("{:03}/{kind}@{revision_id}", spec.number),
          )?;
        }
      }
      Ok(report)
    })
  }
}
