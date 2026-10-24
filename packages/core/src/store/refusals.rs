use std::sync::Arc;

use diesel::pg::PgConnection;

use crate::db::DbPool;
use crate::model::{AuditResult, Caller, StoreError, StoreResult};
use crate::store::organizations::readable_organization;
use crate::store::projects::access_in;
use crate::store::{NewAudit, Store, insert_audit};

#[derive(Debug, Clone, Copy)]
pub enum Scope<'a> {
  Project(&'a str),
  Organization(&'a str),
  Global,
}

#[derive(Debug, Clone)]
pub struct Attempt<'a> {
  pub action: &'a str,
  pub target: String,
  pub scope: Scope<'a>,
  pub expected_revision: Option<i64>,
}

impl<'a> Attempt<'a> {
  pub fn project(action: &'a str, project: &'a str, target: impl Into<String>) -> Self {
    Attempt {
      action,
      target: target.into(),
      scope: Scope::Project(project),
      expected_revision: None,
    }
  }

  pub fn organization(action: &'a str, organization: &'a str, target: impl Into<String>) -> Self {
    Attempt {
      action,
      target: target.into(),
      scope: Scope::Organization(organization),
      expected_revision: None,
    }
  }

  pub fn expecting(mut self, expected_revision: Option<i64>) -> Self {
    self.expected_revision = expected_revision;
    self
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
  pub result: AuditResult,
  pub detail: Option<String>,
}

impl Refusal {
  pub fn from_error(error: &StoreError, attempt: &Attempt<'_>) -> Option<Refusal> {
    match error {
      StoreError::Forbidden => Some(Refusal {
        result: AuditResult::Denied,
        detail: None,
      }),
      StoreError::Conflict { current_revision } => {
        let expected = match attempt.expected_revision {
          Some(n) => format!("r{n}"),
          None => "none".to_string(),
        };
        Some(Refusal {
          result: AuditResult::Conflict,
          detail: Some(format!("expected {expected}, current r{current_revision}")),
        })
      }
      _ => None,
    }
  }
}

pub trait RefusalRecorder: Send + Sync {
  fn record(&self, caller: &Caller, attempt: &Attempt<'_>, refusal: &Refusal) -> StoreResult<()>;
}

pub struct PgRefusalRecorder {
  pool: DbPool,
}

impl PgRefusalRecorder {
  pub fn shared(pool: DbPool) -> Arc<dyn RefusalRecorder> {
    Arc::new(PgRefusalRecorder { pool })
  }
}

fn resolve_scope(
  conn: &mut PgConnection,
  caller: &Caller,
  scope: Scope<'_>,
) -> StoreResult<Option<(Option<i64>, Option<i64>)>> {
  let resolved = match scope {
    Scope::Project(project) => access_in(conn, caller, project)
      .map(|access| (Some(access.organization_id), Some(access.project_id))),
    Scope::Organization(organization) => readable_organization(conn, caller, organization)
      .map(|(organization_id, _)| (Some(organization_id), None)),
    Scope::Global => return Ok(Some((None, None))),
  };
  match resolved {
    Ok(ids) => Ok(Some(ids)),
    Err(StoreError::NotFound(_)) => Ok(None),
    Err(other) => Err(other),
  }
}

impl RefusalRecorder for PgRefusalRecorder {
  fn record(&self, caller: &Caller, attempt: &Attempt<'_>, refusal: &Refusal) -> StoreResult<()> {
    let mut conn = self.pool.get()?;
    let Some((organization_id, project_id)) = resolve_scope(&mut conn, caller, attempt.scope)?
    else {
      return Ok(());
    };
    insert_audit(
      &mut conn,
      NewAudit {
        actor_id: caller.user_id,
        via: &caller.via,
        organization_id,
        project_id,
        action: attempt.action,
        target: &attempt.target,
        result: refusal.result,
        detail: refusal.detail.as_deref(),
      },
    )
  }
}

impl Store {
  pub fn audited<T>(
    &self,
    caller: &Caller,
    attempt: Attempt<'_>,
    f: impl FnOnce(&mut PgConnection) -> StoreResult<T>,
  ) -> StoreResult<T> {
    let outcome = self.with_tx(f);
    if let Err(error) = &outcome
      && let Some(refusal) = Refusal::from_error(error, &attempt)
      && let Err(failure) = self.recorder.record(caller, &attempt, &refusal)
    {
      tracing::error!(
        action = attempt.action,
        target = %attempt.target,
        error = %failure,
        "could not record a refused write"
      );
    }
    outcome
  }
}
