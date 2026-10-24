use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::clients::{CLI_CLIENT_NAME, WEB_APP_CLIENT_NAME};
use crate::markdown::markers::MarkerKind;
use crate::markdown::tasks::parse_tasks;
use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult};
use crate::schema::audit_log;
use crate::store::Store;
use crate::store::projects::{Access, access_in};
use crate::store::views::traceability::{Coverage, build_traceability};
use crate::store::views::{LiveSpec, live_specs};

pub const PERIODS: [i64; 3] = [7, 14, 30];
pub const ACTIVE_AGENT_WINDOW_MINUTES: i64 = 15;
pub const UNKNOWN_CLIENT: &str = "Unknown";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCount {
  pub client: String,
  pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayCompleted {
  pub day: String,
  pub by_client: Vec<ClientCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementCoverage {
  pub spec_number: i64,
  pub covered: usize,
  pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionKind {
  ApprovalWaiting,
  OpenQuestions,
  RequirementWithoutWork,
}

impl AttentionKind {
  pub fn as_str(self) -> &'static str {
    match self {
      AttentionKind::ApprovalWaiting => "approval_waiting",
      AttentionKind::OpenQuestions => "open_questions",
      AttentionKind::RequirementWithoutWork => "requirement_without_work",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionItem {
  pub kind: AttentionKind,
  pub spec_number: i64,
  pub spec_slug: String,
  pub count: usize,
  pub detail: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overview {
  pub days: i64,
  pub active_specs: usize,
  pub tasks_completed: i64,
  pub tasks_completed_previous: i64,
  pub coverage: Option<RequirementCoverage>,
  pub open_markers: usize,
  pub agents_active: usize,
  pub completed_per_day: Vec<DayCompleted>,
  pub attention: Vec<AttentionItem>,
}

fn is_complete(spec: &LiveSpec) -> bool {
  let tasks = spec
    .body(DocumentKind::Tasks)
    .map(|body| parse_tasks(body).tasks)
    .unwrap_or_default();
  spec.row.status == SpecStatus::Approved && !tasks.is_empty() && tasks.iter().all(|t| t.done)
}

fn completed_tasks(
  conn: &mut PgConnection,
  project_id: i64,
  since: DateTime<Utc>,
) -> StoreResult<Vec<(DateTime<Utc>, Option<String>)>> {
  Ok(
    audit_log::table
      .filter(audit_log::project_id.eq(project_id))
      .filter(audit_log::action.eq("task.done"))
      .filter(audit_log::result.eq("ok"))
      .filter(audit_log::created_at.ge(since))
      .select((audit_log::created_at, audit_log::client))
      .load(conn)?,
  )
}

fn agents_active(
  conn: &mut PgConnection,
  project_id: i64,
  now: DateTime<Utc>,
) -> StoreResult<usize> {
  let clients: Vec<Option<String>> = audit_log::table
    .filter(audit_log::project_id.eq(project_id))
    .filter(audit_log::result.eq("ok"))
    .filter(audit_log::created_at.ge(now - Duration::minutes(ACTIVE_AGENT_WINDOW_MINUTES)))
    .select(audit_log::client)
    .distinct()
    .load(conn)?;
  Ok(
    clients
      .into_iter()
      .flatten()
      .filter(|client| client != WEB_APP_CLIENT_NAME && client != CLI_CLIENT_NAME)
      .count(),
  )
}

fn per_day(
  rows: &[(DateTime<Utc>, Option<String>)],
  today: chrono::NaiveDate,
  days: i64,
) -> Vec<DayCompleted> {
  let mut buckets: BTreeMap<chrono::NaiveDate, BTreeMap<String, i64>> = (0..days)
    .map(|back| (today - Duration::days(days - 1 - back), BTreeMap::new()))
    .collect();
  for (at, client) in rows {
    if let Some(bucket) = buckets.get_mut(&at.date_naive()) {
      let client = client.clone().unwrap_or_else(|| UNKNOWN_CLIENT.to_string());
      *bucket.entry(client).or_insert(0) += 1;
    }
  }
  buckets
    .into_iter()
    .map(|(day, counts)| {
      let mut by_client: Vec<ClientCount> = counts
        .into_iter()
        .map(|(client, count)| ClientCount { client, count })
        .collect();
      by_client.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.client.cmp(&b.client)));
      DayCompleted {
        day: day.to_string(),
        by_client,
      }
    })
    .collect()
}

fn attention(specs: &[LiveSpec], active: Option<&LiveSpec>, owner: bool) -> Vec<AttentionItem> {
  let item = |kind, spec: &LiveSpec, count, detail| AttentionItem {
    kind,
    spec_number: spec.row.number,
    spec_slug: spec.row.slug.clone(),
    count,
    detail,
  };
  let mut items = Vec::new();
  if owner {
    items.extend(
      specs
        .iter()
        .filter(|spec| spec.row.status == SpecStatus::Clarified)
        .map(|spec| item(AttentionKind::ApprovalWaiting, spec, 1, Vec::new())),
    );
  }
  for spec in specs {
    let questions = spec
      .markers()
      .iter()
      .filter(|m| m.kind == MarkerKind::Clarification)
      .count();
    if questions > 0 {
      items.push(item(
        AttentionKind::OpenQuestions,
        spec,
        questions,
        Vec::new(),
      ));
    }
  }
  if let Some(spec) = active {
    let uncovered: Vec<String> = build_traceability(
      spec.body(DocumentKind::Spec),
      spec.body(DocumentKind::Tasks),
    )
    .requirements
    .into_iter()
    .filter(|r| r.status == Coverage::NoTasks)
    .map(|r| r.id)
    .collect();
    if !uncovered.is_empty() {
      items.push(item(
        AttentionKind::RequirementWithoutWork,
        spec,
        uncovered.len(),
        uncovered,
      ));
    }
  }
  items
}

pub(crate) fn overview_in(
  conn: &mut PgConnection,
  access: &Access,
  days: i64,
  now: DateTime<Utc>,
) -> StoreResult<Overview> {
  let today = now.date_naive();
  let period_start = (today - Duration::days(days - 1))
    .and_time(NaiveTime::MIN)
    .and_utc();
  let previous_start = period_start - Duration::days(days);
  let completed = completed_tasks(conn, access.project_id, previous_start)?;
  let tasks_completed = completed
    .iter()
    .filter(|(at, _)| *at >= period_start)
    .count() as i64;
  let specs = live_specs(conn, access.project_id)?;
  let active = specs.last();
  let coverage = active.map(|spec| {
    let requirements = build_traceability(
      spec.body(DocumentKind::Spec),
      spec.body(DocumentKind::Tasks),
    )
    .requirements;
    RequirementCoverage {
      spec_number: spec.row.number,
      covered: requirements
        .iter()
        .filter(|r| r.status == Coverage::Covered)
        .count(),
      total: requirements.len(),
    }
  });
  Ok(Overview {
    days,
    active_specs: specs.iter().filter(|spec| !is_complete(spec)).count(),
    tasks_completed,
    tasks_completed_previous: completed.len() as i64 - tasks_completed,
    coverage,
    open_markers: specs.iter().map(|spec| spec.markers().len()).sum(),
    agents_active: agents_active(conn, access.project_id, now)?,
    completed_per_day: per_day(&completed, today, days),
    attention: attention(&specs, active, access.is_owner()),
  })
}

impl Store {
  /// KPIs, tasks completed per day and client, and attention items of a project for a period of 7, 14 or 30 days.
  pub fn project_dashboard(
    &self,
    caller: &Caller,
    project: &str,
    days: i64,
  ) -> StoreResult<Overview> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      if !PERIODS.contains(&days) {
        return Err(StoreError::Validation(format!(
          "unknown period {days}; valid periods: 7, 14, 30"
        )));
      }
      overview_in(conn, &access, days, Utc::now())
    })
  }
}
