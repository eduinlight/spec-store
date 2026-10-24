use std::collections::HashMap;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::address::ProjectRef;
use crate::markdown::markers::scan_markers;
use crate::markdown::tasks::parse_tasks;
use crate::model::{Caller, DocumentKind, Role, SpecStatus, StoreResult};
use crate::schema::{audit_log, documents, iso_utc, revisions, specs, users};
use crate::store::Store;
use crate::store::organizations::visible_organization;
use crate::store::projects::visible_projects_in;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstitutionSummary {
  pub version: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSpecSummary {
  pub number: i64,
  pub slug: String,
  pub status: SpecStatus,
  pub tasks_done: usize,
  pub tasks_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastActivity {
  pub actor: String,
  pub client: Option<String>,
  pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
  pub address: ProjectRef,
  pub slug: String,
  pub name: String,
  pub role: Option<Role>,
  pub archived: bool,
  pub constitution: Option<ConstitutionSummary>,
  pub active_spec: Option<ActiveSpecSummary>,
  pub last_activity: Option<LastActivity>,
  pub open_markers: usize,
}

type ActiveRow = (i64, i64, i64, String, String);

fn constitutions(
  conn: &mut PgConnection,
  project_ids: &[i64],
) -> StoreResult<HashMap<i64, ConstitutionSummary>> {
  let rows: Vec<(i64, Option<String>, String)> = documents::table
    .inner_join(revisions::table.on(documents::current_revision_id.eq(revisions::id.nullable())))
    .filter(documents::project_id.eq_any(project_ids))
    .filter(documents::kind.eq(DocumentKind::Constitution.as_str()))
    .select((
      documents::project_id,
      revisions::version,
      iso_utc(revisions::created_at),
    ))
    .load(conn)?;
  Ok(
    rows
      .into_iter()
      .map(|(project_id, version, updated_at)| {
        (
          project_id,
          ConstitutionSummary {
            version: version.unwrap_or_default(),
            updated_at,
          },
        )
      })
      .collect(),
  )
}

fn active_specs(
  conn: &mut PgConnection,
  project_ids: &[i64],
) -> StoreResult<HashMap<i64, ActiveRow>> {
  let rows: Vec<ActiveRow> = specs::table
    .filter(specs::project_id.eq_any(project_ids))
    .filter(specs::status.ne(SpecStatus::Archived.as_str()))
    .select((
      specs::project_id,
      specs::id,
      specs::number,
      specs::slug,
      specs::status,
    ))
    .order((specs::project_id, specs::number.desc()))
    .load(conn)?;
  let mut active = HashMap::new();
  for row in rows {
    active.entry(row.0).or_insert(row);
  }
  Ok(active)
}

fn spec_documents(
  conn: &mut PgConnection,
  spec_ids: &[i64],
) -> StoreResult<HashMap<i64, Vec<(String, String)>>> {
  let rows: Vec<(i64, String, String)> = documents::table
    .inner_join(revisions::table.on(documents::current_revision_id.eq(revisions::id.nullable())))
    .filter(documents::spec_id.eq_any(spec_ids))
    .select((
      documents::spec_id.assume_not_null(),
      documents::kind,
      revisions::body,
    ))
    .load(conn)?;
  let mut by_spec: HashMap<i64, Vec<(String, String)>> = HashMap::new();
  for (spec_id, kind, body) in rows {
    by_spec.entry(spec_id).or_default().push((kind, body));
  }
  Ok(by_spec)
}

fn last_activities(
  conn: &mut PgConnection,
  project_ids: &[i64],
) -> StoreResult<HashMap<i64, LastActivity>> {
  let rows: Vec<(Option<i64>, String, Option<String>, String)> = audit_log::table
    .inner_join(users::table)
    .filter(audit_log::project_id.eq_any(project_ids))
    .filter(audit_log::result.eq("ok"))
    .distinct_on(audit_log::project_id)
    .order((audit_log::project_id, audit_log::id.desc()))
    .select((
      audit_log::project_id,
      users::handle,
      audit_log::client,
      iso_utc(audit_log::created_at),
    ))
    .load(conn)?;
  Ok(
    rows
      .into_iter()
      .filter_map(|(project_id, actor, client, at)| {
        project_id.map(|id| (id, LastActivity { actor, client, at }))
      })
      .collect(),
  )
}

impl Store {
  pub fn project_summaries(
    &self,
    caller: &Caller,
    organization: &str,
  ) -> StoreResult<Vec<ProjectSummary>> {
    self.with_conn(|conn| {
      visible_organization(conn, caller, organization)?;
      let visible = visible_projects_in(conn, caller, Some(organization))?;
      let project_ids: Vec<i64> = visible.iter().map(|p| p.id).collect();
      let mut constitutions = constitutions(conn, &project_ids)?;
      let mut activities = last_activities(conn, &project_ids)?;
      let active = active_specs(conn, &project_ids)?;
      let spec_ids: Vec<i64> = active.values().map(|row| row.1).collect();
      let mut bodies = spec_documents(conn, &spec_ids)?;
      visible
        .into_iter()
        .map(|project| {
          let (active_spec, open_markers) = match active.get(&project.id) {
            Some((_, spec_id, number, slug, status)) => {
              let documents = bodies.remove(spec_id).unwrap_or_default();
              let tasks = documents
                .iter()
                .find(|(kind, _)| kind == DocumentKind::Tasks.as_str())
                .map(|(_, body)| parse_tasks(body).tasks)
                .unwrap_or_default();
              let markers = documents
                .iter()
                .map(|(_, body)| scan_markers(body).len())
                .sum();
              (
                Some(ActiveSpecSummary {
                  number: *number,
                  slug: slug.clone(),
                  status: status.parse()?,
                  tasks_done: tasks.iter().filter(|t| t.done).count(),
                  tasks_total: tasks.len(),
                }),
                markers,
              )
            }
            None => (None, 0),
          };
          Ok(ProjectSummary {
            slug: project.address.project.clone(),
            name: project.name,
            role: project.role,
            archived: project.archived,
            constitution: constitutions.remove(&project.id),
            last_activity: activities.remove(&project.id),
            address: project.address,
            active_spec,
            open_markers,
          })
        })
        .collect()
    })
  }
}
