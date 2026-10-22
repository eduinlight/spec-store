use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult};
use crate::schema::specs;
use crate::store::documents::{append_revision, current_revision, document_id, ensure_document};
use crate::store::projects::access_in;
use crate::store::specs::{branch_name, spec_by_number, spec_rows_in};
use crate::store::{Store, audit};
use crate::tree::{
  CONSTITUTION_PATH, ExportFile, ProjectTree, TreeSpec, status_word, version_line,
};

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

fn current_body(
  conn: &mut SqliteConnection,
  project_id: i64,
  spec_id: Option<i64>,
  kind: DocumentKind,
) -> StoreResult<Option<String>> {
  match document_id(conn, project_id, spec_id, kind)? {
    Some(doc_id) => Ok(current_revision(conn, doc_id)?.map(|(body, _)| body)),
    None => Ok(None),
  }
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
  conn: &mut SqliteConnection,
  project_id: i64,
  project: &str,
  spec: &TreeSpec,
) -> StoreResult<Vec<String>> {
  let mut problems = Vec::new();
  let number_taken: i64 = specs::table
    .filter(specs::project_id.eq(project_id))
    .filter(specs::number.eq(spec.number))
    .count()
    .get_result(conn)?;
  if number_taken > 0 {
    problems.push(format!(
      "specs/{}: spec {:03} already exists in project {project:?}",
      spec.folder, spec.number
    ));
  }
  let slug_taken: i64 = specs::table
    .filter(specs::project_id.eq(project_id))
    .filter(specs::slug.eq(&spec.slug))
    .count()
    .get_result(conn)?;
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
        let spec_id: i64 = diesel::insert_into(specs::table)
          .values((
            specs::project_id.eq(access.project_id),
            specs::number.eq(spec.number),
            specs::slug.eq(&spec.slug),
            specs::status.eq(status.as_str()),
            specs::branch.eq(&branch),
          ))
          .returning(specs::id)
          .get_result(tx)?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusNotice {
  pub branch: String,
  pub stored: SpecStatus,
  pub written: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectExport {
  pub files: Vec<ExportFile>,
  pub notices: Vec<StatusNotice>,
}

impl Store {
  pub fn export_documents(
    &self,
    caller: &Caller,
    project: &str,
    spec: Option<i64>,
  ) -> StoreResult<ProjectExport> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let mut export = ProjectExport::default();
      let specs = match spec {
        Some(number) => vec![spec_by_number(conn, access.project_id, project, number)?],
        None => {
          if let Some(body) =
            current_body(conn, access.project_id, None, DocumentKind::Constitution)?
          {
            export.files.push(ExportFile {
              relative_path: CONSTITUTION_PATH.to_string(),
              body,
            });
          }
          spec_rows_in(conn, access.project_id)?
        }
      };
      for row in specs {
        for kind in DocumentKind::SPEC_KINDS {
          let Some(body) = current_body(conn, access.project_id, Some(row.id), kind)? else {
            continue;
          };
          if kind == DocumentKind::Spec
            && let Some(written) = status_word(&body)
            && written != row.status.as_str()
          {
            export.notices.push(StatusNotice {
              branch: row.branch.clone(),
              stored: row.status,
              written: written.to_string(),
            });
          }
          export.files.push(ExportFile {
            relative_path: format!("specs/{}/{kind}.md", row.branch),
            body,
          });
        }
      }
      Ok(export)
    })
  }
}
