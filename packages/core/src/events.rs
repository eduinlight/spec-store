use std::cell::RefCell;

use tokio::sync::broadcast;

use crate::model::AuditResult;

/// Notification that an audit entry of a project was committed; it carries no document content.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectEvent {
  pub project_id: i64,
  pub organization_id: i64,
  pub project_address: String,
  pub audit_id: i64,
  pub action: String,
  pub result: AuditResult,
}

/// Port through which committed audit entries reach live viewers; implementations never fail the caller.
pub trait EventPublisher: Send + Sync {
  fn publish(&self, event: &ProjectEvent);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopPublisher;

impl EventPublisher for NoopPublisher {
  fn publish(&self, _event: &ProjectEvent) {}
}

/// Delivers events to subscribers of the same process; used by single-instance runs and tests.
#[derive(Debug, Clone)]
pub struct InProcessPublisher {
  sender: broadcast::Sender<ProjectEvent>,
}

impl InProcessPublisher {
  pub fn new(capacity: usize) -> Self {
    InProcessPublisher {
      sender: broadcast::channel(capacity).0,
    }
  }

  pub fn subscribe(&self) -> broadcast::Receiver<ProjectEvent> {
    self.sender.subscribe()
  }
}

impl EventPublisher for InProcessPublisher {
  fn publish(&self, event: &ProjectEvent) {
    let _ = self.sender.send(event.clone());
  }
}

thread_local! {
  static PENDING: RefCell<Vec<ProjectEvent>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn defer(event: ProjectEvent) {
  PENDING.with(|pending| pending.borrow_mut().push(event));
}

pub(crate) fn take_pending() -> Vec<ProjectEvent> {
  PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()))
}

pub(crate) fn discard_pending() {
  PENDING.with(|pending| pending.borrow_mut().clear());
}
