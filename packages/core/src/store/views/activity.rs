use std::str::FromStr;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{AuditResult, Caller, StoreError, StoreResult};
use crate::schema::{audit_log, iso_utc, users};
use crate::store::Store;
use crate::store::projects::access_in;

pub const DEFAULT_ACTIVITY_LIMIT: usize = 50;

const TASK_ACTIONS: [&str; 1] = ["task.done"];
const DOCUMENT_ACTIONS: [&str; 7] = [
  "document.put",
  "constitution.put",
  "spec.create",
  "spec.status",
  "spec.import",
  "project.import",
  "project.copy",
];
const SECURITY_ACTIONS: [&str; 4] = [
  "project.create",
  "project.archive",
  "project.restore",
  "project.repo",
];
const SECURITY_PREFIXES: [&str; 3] = ["member.%", "organization.%", "invitation.%"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityFilter {
  All,
  Tasks,
  Documents,
  Security,
}

impl ActivityFilter {
  pub fn as_str(self) -> &'static str {
    match self {
      ActivityFilter::All => "all",
      ActivityFilter::Tasks => "tasks",
      ActivityFilter::Documents => "documents",
      ActivityFilter::Security => "security",
    }
  }
}

impl FromStr for ActivityFilter {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    [
      ActivityFilter::All,
      ActivityFilter::Tasks,
      ActivityFilter::Documents,
      ActivityFilter::Security,
    ]
    .into_iter()
    .find(|f| f.as_str() == s)
    .ok_or_else(|| {
      StoreError::Validation(format!(
        "unknown filter {s:?}; valid filters: all, tasks, documents, security"
      ))
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRow {
  pub audit_id: i64,
  pub actor: String,
  pub client: Option<String>,
  pub action: String,
  pub target: String,
  pub result: AuditResult,
  pub detail: Option<String>,
  pub at: String,
  pub summary_key: String,
  pub resolved_by_audit_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActivityPage {
  pub rows: Vec<ActivityRow>,
  pub has_more: bool,
}

type RawRow = (
  i64,
  i64,
  String,
  Option<String>,
  String,
  String,
  String,
  Option<String>,
  String,
);

fn document_of(target: &str) -> &str {
  target.split('@').next().unwrap_or(target)
}

fn summary_key(action: &str, result: AuditResult, resolved: bool) -> String {
  match result {
    AuditResult::Ok => action.to_string(),
    AuditResult::Denied => "denied".to_string(),
    AuditResult::Conflict if resolved => "conflict_resolved".to_string(),
    AuditResult::Conflict => "conflict".to_string(),
  }
}

fn raw_rows(
  conn: &mut PgConnection,
  project_id: i64,
  filter: ActivityFilter,
  before: Option<i64>,
  limit: i64,
) -> StoreResult<Vec<RawRow>> {
  let mut query = audit_log::table
    .inner_join(users::table)
    .filter(audit_log::project_id.eq(project_id))
    .select((
      audit_log::id,
      audit_log::actor_id,
      users::handle,
      audit_log::client,
      audit_log::action,
      audit_log::target,
      audit_log::result,
      audit_log::detail,
      iso_utc(audit_log::created_at),
    ))
    .order(audit_log::id.desc())
    .limit(limit)
    .into_boxed();
  query = match filter {
    ActivityFilter::All => query,
    ActivityFilter::Tasks => query.filter(audit_log::action.eq_any(TASK_ACTIONS)),
    ActivityFilter::Documents => query.filter(audit_log::action.eq_any(DOCUMENT_ACTIONS)),
    ActivityFilter::Security => query.filter(
      audit_log::result
        .eq(AuditResult::Denied.as_str())
        .or(audit_log::action.like(SECURITY_PREFIXES[0]))
        .or(audit_log::action.like(SECURITY_PREFIXES[1]))
        .or(audit_log::action.like(SECURITY_PREFIXES[2]))
        .or(audit_log::action.eq_any(SECURITY_ACTIONS)),
    ),
  };
  if let Some(before) = before {
    query = query.filter(audit_log::id.lt(before));
  }
  Ok(query.load(conn)?)
}

fn resolutions(
  conn: &mut PgConnection,
  project_id: i64,
  rows: &[RawRow],
) -> StoreResult<Vec<(i64, i64)>> {
  let conflicts: Vec<&RawRow> = rows
    .iter()
    .filter(|row| row.6 == AuditResult::Conflict.as_str() && row.4 == "document.put")
    .collect();
  let Some(oldest) = conflicts.iter().map(|row| row.0).min() else {
    return Ok(Vec::new());
  };
  let actors: Vec<i64> = conflicts.iter().map(|row| row.1).collect();
  let saved: Vec<(i64, i64, String)> = audit_log::table
    .filter(audit_log::project_id.eq(project_id))
    .filter(audit_log::action.eq("document.put"))
    .filter(audit_log::result.eq(AuditResult::Ok.as_str()))
    .filter(audit_log::id.gt(oldest))
    .filter(audit_log::actor_id.eq_any(actors))
    .select((audit_log::id, audit_log::actor_id, audit_log::target))
    .order(audit_log::id.asc())
    .load(conn)?;
  Ok(
    conflicts
      .iter()
      .filter_map(|conflict| {
        saved
          .iter()
          .find(|(id, actor, target)| {
            *id > conflict.0
              && *actor == conflict.1
              && document_of(target) == document_of(&conflict.5)
          })
          .map(|(id, _, _)| (conflict.0, *id))
      })
      .collect(),
  )
}

impl Store {
  /// Activity of a project newest first, filtered, with conflicts followed by a saved write marked as resolved.
  pub fn project_activity(
    &self,
    caller: &Caller,
    project: &str,
    filter: ActivityFilter,
    before: Option<i64>,
    limit: usize,
  ) -> StoreResult<ActivityPage> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let mut raw = raw_rows(conn, access.project_id, filter, before, limit as i64 + 1)?;
      let has_more = raw.len() > limit;
      raw.truncate(limit);
      let resolved = resolutions(conn, access.project_id, &raw)?;
      let rows = raw
        .into_iter()
        .map(
          |(audit_id, _, actor, client, action, target, result, detail, at)| {
            let result: AuditResult = result.parse()?;
            let resolved_by = resolved
              .iter()
              .find(|(conflict, _)| *conflict == audit_id)
              .map(|(_, saved)| *saved);
            Ok(ActivityRow {
              summary_key: summary_key(&action, result, resolved_by.is_some()),
              resolved_by_audit_id: resolved_by,
              audit_id,
              actor,
              client,
              action,
              target,
              result,
              detail,
              at,
            })
          },
        )
        .collect::<StoreResult<Vec<ActivityRow>>>()?;
      Ok(ActivityPage { rows, has_more })
    })
  }
}
