mod common;

use std::sync::{Arc, Mutex, OnceLock};

use sdd_vault_core::events::{EventPublisher, InProcessPublisher, NoopPublisher, ProjectEvent};
use sdd_vault_core::model::{AuditResult, Caller, DocumentKind, Role, StoreError};
use sdd_vault_core::store::{Store, audit};

struct Probe {
  store: OnceLock<Store>,
  seen: Mutex<Vec<(ProjectEvent, bool)>>,
}

impl Probe {
  fn new() -> Arc<Probe> {
    Arc::new(Probe {
      store: OnceLock::new(),
      seen: Mutex::new(Vec::new()),
    })
  }

  fn events(&self) -> Vec<ProjectEvent> {
    self
      .seen
      .lock()
      .unwrap()
      .iter()
      .map(|(e, _)| e.clone())
      .collect()
  }

  fn all_visible_when_published(&self) -> bool {
    self
      .seen
      .lock()
      .unwrap()
      .iter()
      .all(|(_, visible)| *visible)
  }
}

impl EventPublisher for Probe {
  fn publish(&self, event: &ProjectEvent) {
    let visible = match self.store.get() {
      Some(store) => {
        common::int(
          store,
          &format!(
            "SELECT count(*) FROM audit_log WHERE id = {}",
            event.audit_id
          ),
        ) == 1
      }
      None => true,
    };
    self.seen.lock().unwrap().push((event.clone(), visible));
  }
}

struct Fixture {
  store: Store,
  _db: sdd_vault_testkit::TestDb,
  root: Caller,
  editor: Caller,
  reader: Caller,
  outsider: Caller,
}

fn fixture_with(publisher: Arc<dyn EventPublisher>) -> Fixture {
  let db = common::test_db();
  let store = Store::open_with_size(&db.url, 4)
    .unwrap()
    .with_publisher(publisher);
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let editor = common::person(&store, "editor");
  let reader = common::person(&store, "reader");
  common::join(&store, &root, common::ORG, &editor);
  common::join(&store, &root, common::ORG, &reader);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  let outsider = common::person(&store, "outsider");
  Fixture {
    store,
    _db: db,
    root,
    editor,
    reader,
    outsider,
  }
}

fn probed() -> (Fixture, Arc<Probe>) {
  let probe = Probe::new();
  let f = fixture_with(probe.clone());
  probe.store.set(f.store.clone()).ok().unwrap();
  (f, probe)
}

fn project_id(f: &Fixture) -> i64 {
  common::int(&f.store, "SELECT id FROM projects WHERE slug = 'demo'")
}

#[test]
fn a_committed_write_publishes_one_event_after_the_commit() {
  let (f, probe) = probed();
  let before = probe.events().len();
  f.store
    .create_spec(&f.root, "acme/demo", "billing")
    .unwrap();
  let events = probe.events();
  assert_eq!(events.len(), before + 1);
  let event = events.last().unwrap();
  assert_eq!(event.project_address, "acme/demo");
  assert_eq!(event.project_id, project_id(&f));
  assert_eq!(
    event.organization_id,
    common::int(&f.store, "SELECT id FROM organizations WHERE slug = 'acme'")
  );
  assert_eq!(event.action, "spec.create");
  assert_eq!(event.result, AuditResult::Ok);
  assert_eq!(
    event.audit_id,
    common::int(&f.store, "SELECT max(id) FROM audit_log")
  );
  assert!(probe.all_visible_when_published());
}

#[test]
fn a_rolled_back_transaction_publishes_nothing_and_leaves_nothing_behind() {
  let (f, probe) = probed();
  let before = probe.events().len();
  let id = project_id(&f);
  let failed: Result<(), StoreError> = f.store.with_tx(|tx| {
    audit(tx, &f.root, Some(id), "spec.create", "doomed")?;
    Err(StoreError::Validation("rolled back".into()))
  });
  assert!(failed.is_err());
  assert_eq!(probe.events().len(), before);
  f.store.create_spec(&f.root, "acme/demo", "real").unwrap();
  let events = probe.events();
  assert_eq!(events.len(), before + 1);
  assert_eq!(events.last().unwrap().action, "spec.create");
}

#[test]
fn every_audit_entry_of_one_transaction_is_published_in_order_after_the_commit() {
  let (f, probe) = probed();
  let before = probe.events().len();
  let id = project_id(&f);
  f.store
    .with_tx(|tx| {
      audit(tx, &f.root, Some(id), "spec.create", "one")?;
      audit(tx, &f.root, Some(id), "spec.status", "two")
    })
    .unwrap();
  let events = probe.events();
  let actions: Vec<&str> = events[before..].iter().map(|e| e.action.as_str()).collect();
  assert_eq!(actions, ["spec.create", "spec.status"]);
  assert!(events[before].audit_id < events[before + 1].audit_id);
  assert!(probe.all_visible_when_published());
}

#[test]
fn refused_writes_publish_their_refusal() {
  let (f, probe) = probed();
  f.store
    .create_spec(&f.root, "acme/demo", "billing")
    .unwrap();
  let before = probe.events().len();
  let denied = f.store.put_document(
    &f.reader,
    "acme/demo",
    1,
    DocumentKind::Plan,
    "# Plan",
    "edit",
    None,
  );
  assert!(matches!(denied, Err(StoreError::Forbidden)));
  let first = f
    .store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Plan,
      "# Plan",
      "edit",
      None,
    )
    .unwrap();
  let conflict = f.store.put_document(
    &f.editor,
    "acme/demo",
    1,
    DocumentKind::Plan,
    "# Plan 2",
    "edit",
    Some(first + 100),
  );
  assert!(matches!(conflict, Err(StoreError::Conflict { .. })));
  let events = probe.events();
  let seen: Vec<(&str, AuditResult)> = events[before..]
    .iter()
    .map(|e| (e.action.as_str(), e.result))
    .collect();
  assert_eq!(
    seen,
    [
      ("document.put", AuditResult::Denied),
      ("document.put", AuditResult::Ok),
      ("document.put", AuditResult::Conflict),
    ]
  );
  assert!(probe.all_visible_when_published());
}

#[test]
fn writes_that_leave_no_audit_entry_or_no_project_publish_nothing() {
  let (f, probe) = probed();
  let before = probe.events().len();
  let missing = f.store.put_document(
    &f.outsider,
    "acme/demo",
    1,
    DocumentKind::Plan,
    "# Plan",
    "edit",
    None,
  );
  assert!(matches!(missing, Err(StoreError::NotFound(_))));
  f.store
    .create_organization(&f.outsider, "other", "Other")
    .unwrap();
  assert_eq!(probe.events().len(), before);
}

#[test]
fn the_default_publisher_does_nothing_and_writes_still_succeed() {
  let db = common::test_db();
  let store = Store::open_with_size(&db.url, 4).unwrap();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store.create_spec(&root, "acme/demo", "billing").unwrap();
  let event = ProjectEvent {
    project_id: 1,
    organization_id: 1,
    project_address: "acme/demo".into(),
    audit_id: 1,
    action: "spec.create".into(),
    result: AuditResult::Ok,
  };
  NoopPublisher.publish(&event);
  store.publisher().publish(&event);
}

#[tokio::test]
async fn the_in_process_publisher_delivers_to_every_subscriber() {
  let hub = InProcessPublisher::new(64);
  let mut first = hub.subscribe();
  let mut second = hub.subscribe();
  let publisher: Arc<dyn EventPublisher> = Arc::new(hub);
  let f = tokio::task::spawn_blocking(move || fixture_with(publisher))
    .await
    .unwrap();
  let mut seen = Vec::new();
  while let Ok(event) = first.try_recv() {
    seen.push(event);
  }
  assert!(seen.iter().any(|e| e.action == "project.create"));
  let last = seen.last().unwrap().clone();
  assert_eq!(last.project_address, "acme/demo");
  let mut other = Vec::new();
  while let Ok(event) = second.try_recv() {
    other.push(event);
  }
  assert_eq!(seen, other);
  drop(f);
}

#[test]
fn publishing_without_subscribers_is_harmless() {
  let hub = InProcessPublisher::new(4);
  hub.publish(&ProjectEvent {
    project_id: 1,
    organization_id: 1,
    project_address: "acme/demo".into(),
    audit_id: 1,
    action: "spec.create".into(),
    result: AuditResult::Ok,
  });
}

#[test]
fn events_travel_as_json_with_the_result_in_words() {
  let event = ProjectEvent {
    project_id: 7,
    organization_id: 3,
    project_address: "acme/demo".into(),
    audit_id: 42,
    action: "task.done".into(),
    result: AuditResult::Conflict,
  };
  let json = serde_json::to_value(&event).unwrap();
  assert_eq!(
    json,
    serde_json::json!({
      "project_id": 7,
      "organization_id": 3,
      "project_address": "acme/demo",
      "audit_id": 42,
      "action": "task.done",
      "result": "conflict"
    })
  );
  let back: ProjectEvent = serde_json::from_value(json).unwrap();
  assert_eq!(back, event);
}
