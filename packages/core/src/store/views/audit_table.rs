use std::str::FromStr;

use chrono::{Duration, Utc};
use diesel::dsl::count_star;
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel::sql_types::Bool;

use crate::model::{AuditResult, Caller, OrgRole, StoreError, StoreResult};
use crate::schema::{audit_log, iso_utc, organizations, projects, users};
use crate::store::organizations::readable_organization;
use crate::store::projects::access_in;
use crate::store::{AuditEntry, AuditRow, Store, audit_entry};

pub const PAGE_SIZES: [u32; 4] = [10, 25, 50, 100];

#[derive(Debug, Clone, Copy)]
pub enum AuditScope<'a> {
  Project(&'a str),
  Organization(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Period {
  Day,
  Week,
  Month,
  #[default]
  All,
}

impl Period {
  fn window(self) -> Option<Duration> {
    match self {
      Period::Day => Some(Duration::hours(24)),
      Period::Week => Some(Duration::days(7)),
      Period::Month => Some(Duration::days(30)),
      Period::All => None,
    }
  }
}

impl FromStr for Period {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "day" => Ok(Period::Day),
      "week" => Ok(Period::Week),
      "month" => Ok(Period::Month),
      "all" => Ok(Period::All),
      other => Err(StoreError::Validation(format!(
        "unknown period {other:?}; valid periods: day, week, month, all"
      ))),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
  #[default]
  Newest,
  Oldest,
}

impl FromStr for SortOrder {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "newest" => Ok(SortOrder::Newest),
      "oldest" => Ok(SortOrder::Oldest),
      other => Err(StoreError::Validation(format!(
        "unknown sort {other:?}; valid sorts: newest, oldest"
      ))),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditQuery {
  pub text: Option<String>,
  pub actors: Vec<String>,
  pub clients: Vec<String>,
  pub actions: Vec<String>,
  pub results: Vec<AuditResult>,
  pub period: Period,
  pub sort: SortOrder,
  pub page: u32,
  pub page_size: u32,
}

impl Default for AuditQuery {
  fn default() -> Self {
    AuditQuery {
      text: None,
      actors: Vec::new(),
      clients: Vec::new(),
      actions: Vec::new(),
      results: Vec::new(),
      period: Period::All,
      sort: SortOrder::Newest,
      page: 1,
      page_size: 25,
    }
  }
}

impl AuditQuery {
  fn validate(&self) -> StoreResult<()> {
    if self.page < 1 {
      return Err(StoreError::Validation("page starts at 1".into()));
    }
    if !PAGE_SIZES.contains(&self.page_size) {
      return Err(StoreError::Validation(
        "page_size must be 10, 25, 50 or 100".into(),
      ));
    }
    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetCount {
  pub value: String,
  pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditFacets {
  pub actors: Vec<FacetCount>,
  pub clients: Vec<FacetCount>,
  pub actions: Vec<FacetCount>,
  pub results: Vec<FacetCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditTable {
  pub entries: Vec<AuditEntry>,
  pub total: i64,
  pub page: u32,
  pub page_size: u32,
  pub facets: AuditFacets,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Facet {
  Actors,
  Clients,
  Actions,
  Results,
}

#[derive(Clone, Copy)]
struct ScopeIds {
  organization_id: i64,
  project_id: Option<i64>,
}

type Predicate = Box<dyn BoxableExpression<audit_log::table, Pg, SqlType = Bool>>;

fn authorize(
  conn: &mut PgConnection,
  caller: &Caller,
  scope: AuditScope<'_>,
) -> StoreResult<ScopeIds> {
  match scope {
    AuditScope::Project(address) => {
      let access = access_in(conn, caller, address)?;
      if !(access.is_owner() || access.is_operator) {
        return Err(StoreError::Forbidden);
      }
      Ok(ScopeIds {
        organization_id: access.organization_id,
        project_id: Some(access.project_id),
      })
    }
    AuditScope::Organization(slug) => {
      let (organization_id, role) = readable_organization(conn, caller, slug)?;
      if role != Some(OrgRole::Owner) && !caller.is_operator {
        return Err(StoreError::Forbidden);
      }
      Ok(ScopeIds {
        organization_id,
        project_id: None,
      })
    }
  }
}

fn escape_like(text: &str) -> String {
  let mut escaped = String::with_capacity(text.len());
  for c in text.chars() {
    if matches!(c, '\\' | '%' | '_') {
      escaped.push('\\');
    }
    escaped.push(c);
  }
  escaped
}

fn scope_predicate(scope: ScopeIds) -> Predicate {
  match scope.project_id {
    Some(project_id) => Box::new(audit_log::project_id.eq(project_id).assume_not_null()),
    None => Box::new(
      audit_log::organization_id
        .eq(scope.organization_id)
        .assume_not_null(),
    ),
  }
}

fn predicate(scope: ScopeIds, query: &AuditQuery, skip: Option<Facet>) -> Predicate {
  let mut all = scope_predicate(scope);
  if let Some(text) = query
    .text
    .as_deref()
    .map(str::trim)
    .filter(|t| !t.is_empty())
  {
    let pattern = format!("%{}%", escape_like(text));
    let matching = audit_log::target
      .ilike(pattern.clone())
      .nullable()
      .or(audit_log::client.ilike(pattern.clone()))
      .or(audit_log::origin.ilike(pattern))
      .assume_not_null();
    all = Box::new(all.and(matching));
  }
  if let Some(window) = query.period.window() {
    all = Box::new(all.and(audit_log::created_at.ge(Utc::now() - window)));
  }
  if skip != Some(Facet::Actors) && !query.actors.is_empty() {
    all = Box::new(
      all.and(
        audit_log::actor_id.eq_any(
          users::table
            .filter(users::handle.eq_any(query.actors.clone()))
            .select(users::id),
        ),
      ),
    );
  }
  if skip != Some(Facet::Clients) && !query.clients.is_empty() {
    all = Box::new(
      all.and(
        audit_log::client
          .eq_any(query.clients.clone())
          .assume_not_null(),
      ),
    );
  }
  if skip != Some(Facet::Actions) && !query.actions.is_empty() {
    all = Box::new(all.and(audit_log::action.eq_any(query.actions.clone())));
  }
  if skip != Some(Facet::Results) && !query.results.is_empty() {
    let results: Vec<String> = query
      .results
      .iter()
      .map(|r| r.as_str().to_string())
      .collect();
    all = Box::new(all.and(audit_log::result.eq_any(results)));
  }
  all
}

fn counted(rows: Vec<(String, i64)>) -> Vec<FacetCount> {
  rows
    .into_iter()
    .map(|(value, count)| FacetCount { value, count })
    .collect()
}

fn actor_counts(
  conn: &mut PgConnection,
  scope: ScopeIds,
  query: &AuditQuery,
) -> StoreResult<Vec<(String, i64)>> {
  let by_id: Vec<(i64, i64)> = audit_log::table
    .filter(predicate(scope, query, Some(Facet::Actors)))
    .group_by(audit_log::actor_id)
    .select((audit_log::actor_id, count_star()))
    .load(conn)?;
  let ids: Vec<i64> = by_id.iter().map(|(id, _)| *id).collect();
  let handles: Vec<(i64, String)> = users::table
    .filter(users::id.eq_any(ids))
    .select((users::id, users::handle))
    .load(conn)?;
  let mut counted: Vec<(String, i64)> = by_id
    .into_iter()
    .filter_map(|(id, count)| {
      handles
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, handle)| (handle.clone(), count))
    })
    .collect();
  counted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
  Ok(counted)
}

fn facets(
  conn: &mut PgConnection,
  scope: ScopeIds,
  query: &AuditQuery,
) -> StoreResult<AuditFacets> {
  macro_rules! grouped {
    ($facet:expr, $column:expr) => {
      audit_log::table
        .filter(predicate(scope, query, Some($facet)))
        .group_by($column)
        .select(($column, count_star()))
        .order((count_star().desc(), $column.asc()))
        .load::<(String, i64)>(conn)?
    };
  }
  let actors = actor_counts(conn, scope, query)?;
  let actions = grouped!(Facet::Actions, audit_log::action);
  let results = grouped!(Facet::Results, audit_log::result);
  let clients: Vec<(Option<String>, i64)> = audit_log::table
    .filter(predicate(scope, query, Some(Facet::Clients)))
    .filter(audit_log::client.is_not_null())
    .group_by(audit_log::client)
    .select((audit_log::client, count_star()))
    .order((count_star().desc(), audit_log::client.asc()))
    .load(conn)?;
  Ok(AuditFacets {
    actors: counted(actors),
    clients: counted(
      clients
        .into_iter()
        .filter_map(|(value, count)| value.map(|v| (v, count)))
        .collect(),
    ),
    actions: counted(actions),
    results: counted(results),
  })
}

fn ordered_ids(
  conn: &mut PgConnection,
  filter: Predicate,
  sort: SortOrder,
  window: Option<(i64, i64)>,
) -> StoreResult<Vec<i64>> {
  let mut statement = audit_log::table
    .filter(filter)
    .select(audit_log::id)
    .into_boxed();
  statement = match sort {
    SortOrder::Newest => statement.order((audit_log::created_at.desc(), audit_log::id.desc())),
    SortOrder::Oldest => statement.order((audit_log::created_at.asc(), audit_log::id.asc())),
  };
  if let Some((limit, offset)) = window {
    statement = statement.limit(limit).offset(offset);
  }
  Ok(statement.load(conn)?)
}

fn entries_in_order(conn: &mut PgConnection, ids: &[i64]) -> StoreResult<Vec<AuditEntry>> {
  if ids.is_empty() {
    return Ok(Vec::new());
  }
  let rows: Vec<AuditRow> = audit_log::table
    .inner_join(users::table)
    .left_join(organizations::table)
    .left_join(projects::table)
    .filter(audit_log::id.eq_any(ids))
    .select((
      audit_log::id,
      users::handle,
      organizations::slug.nullable(),
      projects::slug.nullable(),
      audit_log::action,
      audit_log::target,
      iso_utc(audit_log::created_at),
      audit_log::client,
      audit_log::origin,
      audit_log::result,
      audit_log::detail,
    ))
    .load(conn)?;
  let mut entries = rows
    .into_iter()
    .map(audit_entry)
    .collect::<StoreResult<Vec<AuditEntry>>>()?;
  let position = |id: i64| ids.iter().position(|candidate| *candidate == id);
  entries.sort_by_key(|entry| position(entry.id));
  Ok(entries)
}

impl Store {
  /// One page of the audit trail of a project or organization with the total and the facet counts.
  pub fn audit_table(
    &self,
    caller: &Caller,
    scope: AuditScope<'_>,
    query: &AuditQuery,
  ) -> StoreResult<AuditTable> {
    query.validate()?;
    self.with_conn(|conn| {
      let scope = authorize(conn, caller, scope)?;
      let total = audit_log::table
        .filter(predicate(scope, query, None))
        .count()
        .get_result::<i64>(conn)?;
      let offset = i64::from(query.page - 1) * i64::from(query.page_size);
      let ids = ordered_ids(
        conn,
        predicate(scope, query, None),
        query.sort,
        Some((i64::from(query.page_size), offset)),
      )?;
      let entries = entries_in_order(conn, &ids)?;
      let facets = facets(conn, scope, query)?;
      Ok(AuditTable {
        entries,
        total,
        page: query.page,
        page_size: query.page_size,
        facets,
      })
    })
  }
}
