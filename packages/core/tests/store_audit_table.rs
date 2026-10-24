mod common;

use sdd_vault_core::model::{AuditResult, Caller, StoreError};
use sdd_vault_core::store::Store;
use sdd_vault_core::store::views::audit_table::{
  AuditQuery, AuditScope, FacetCount, Period, SortOrder,
};

struct Fixture {
  store: common::TestStore,
  alice: Caller,
  bob: Caller,
  ops: Caller,
  demo: i64,
  other: i64,
}

struct Row<'a> {
  action: &'a str,
  target: &'a str,
  client: Option<&'a str>,
  origin: Option<&'a str>,
  result: &'a str,
  age_hours: i64,
}

fn row<'a>(
  action: &'a str,
  target: &'a str,
  client: Option<&'a str>,
  origin: Option<&'a str>,
  result: &'a str,
  age_hours: i64,
) -> Row<'a> {
  Row {
    action,
    target,
    client,
    origin,
    result,
    age_hours,
  }
}

fn seed(store: &Store, actor: &Caller, organization: i64, project: Option<i64>, entry: Row<'_>) {
  let Row {
    action,
    target,
    client,
    origin,
    result,
    age_hours,
  } = entry;
  let project = project.map_or("NULL".to_string(), |p| p.to_string());
  let client = client.map_or("NULL".to_string(), |c| format!("'{c}'"));
  let origin = origin.map_or("NULL".to_string(), |o| format!("'{o}'"));
  common::exec(
    store,
    &format!(
      "INSERT INTO audit_log (actor_id, organization_id, project_id, action, target, client, origin, result, created_at) VALUES ({}, {organization}, {project}, '{action}', '{target}', {client}, {origin}, '{result}', now() - interval '{age_hours} hours')",
      actor.user_id
    ),
  );
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let bob = common::person(&store, "bob");
  let ops = common::operator(&store, "ops");
  common::organization(&store, &alice, "acme");
  common::join(&store, &alice, "acme", &bob);
  store.create_project(&alice, "acme/demo", "Demo").unwrap();
  store.create_project(&alice, "acme/other", "Other").unwrap();
  common::exec(&store, "DELETE FROM audit_log");
  let org = common::int(&store, "SELECT id FROM organizations WHERE slug = 'acme'");
  let demo = common::int(&store, "SELECT id FROM projects WHERE slug = 'demo'");
  let other = common::int(&store, "SELECT id FROM projects WHERE slug = 'other'");
  seed(
    &store,
    &alice,
    org,
    Some(demo),
    row(
      "document.write",
      "spec/1/spec",
      Some("claude-code"),
      Some("10.0.0.1"),
      "ok",
      1,
    ),
  );
  seed(
    &store,
    &alice,
    org,
    Some(demo),
    row(
      "document.write",
      "spec/1/plan",
      Some("claude-code"),
      Some("10.0.0.1"),
      "conflict",
      30,
    ),
  );
  seed(
    &store,
    &bob,
    org,
    Some(demo),
    row(
      "document.write",
      "spec/2/spec",
      Some("cursor"),
      Some("192.168.1.9"),
      "denied",
      24 * 5,
    ),
  );
  seed(
    &store,
    &bob,
    org,
    Some(demo),
    row("member.set", "bob", None, None, "ok", 24 * 20),
  );
  seed(
    &store,
    &alice,
    org,
    Some(demo),
    row(
      "member.set",
      "100%_done",
      Some("browser"),
      Some("10.0.0.2"),
      "ok",
      24 * 60,
    ),
  );
  seed(
    &store,
    &alice,
    org,
    Some(other),
    row(
      "document.write",
      "spec/1/spec",
      Some("cursor"),
      Some("10.0.0.3"),
      "ok",
      2,
    ),
  );
  seed(
    &store,
    &alice,
    org,
    None,
    row(
      "organization.rename",
      "acme",
      Some("browser"),
      None,
      "ok",
      3,
    ),
  );
  Fixture {
    store,
    alice,
    bob,
    ops,
    demo,
    other,
  }
}

fn query() -> AuditQuery {
  AuditQuery {
    page_size: 100,
    ..AuditQuery::default()
  }
}

fn targets(entries: &[sdd_vault_core::store::AuditEntry]) -> Vec<&str> {
  entries.iter().map(|e| e.target.as_str()).collect()
}

fn facet(values: &[FacetCount]) -> Vec<(String, i64)> {
  values.iter().map(|f| (f.value.clone(), f.count)).collect()
}

#[test]
fn defaults_list_every_project_entry_newest_first() {
  let f = fixture();
  let table = f
    .store
    .audit_table(&f.alice, AuditScope::Project("acme/demo"), &query())
    .unwrap();
  assert_eq!(table.total, 5);
  assert_eq!(table.page, 1);
  assert_eq!(table.page_size, 100);
  assert_eq!(
    targets(&table.entries),
    vec![
      "spec/1/spec",
      "spec/1/plan",
      "spec/2/spec",
      "bob",
      "100%_done"
    ]
  );
  assert_eq!(table.entries[0].actor, "alice");
  assert_eq!(table.entries[0].project.as_deref(), Some("acme/demo"));
  assert_eq!(table.entries[0].client.as_deref(), Some("claude-code"));
  assert_eq!(table.entries[0].result, AuditResult::Ok);
  assert!(table.entries[0].id > 0);
}

#[test]
fn organization_scope_includes_projects_and_organization_entries() {
  let f = fixture();
  let table = f
    .store
    .audit_table(&f.alice, AuditScope::Organization("acme"), &query())
    .unwrap();
  assert_eq!(table.total, 7);
  assert_eq!(table.entries[0].target, "spec/1/spec");
  assert_eq!(table.entries[1].target, "spec/1/spec");
  assert_eq!(table.entries[1].project.as_deref(), Some("acme/other"));
  assert_eq!(table.entries[2].target, "acme");
  assert_eq!(table.entries[2].project, None);
}

#[test]
fn sort_oldest_reverses_the_order() {
  let f = fixture();
  let table = f
    .store
    .audit_table(
      &f.alice,
      AuditScope::Project("acme/demo"),
      &AuditQuery {
        sort: SortOrder::Oldest,
        ..query()
      },
    )
    .unwrap();
  assert_eq!(
    targets(&table.entries),
    vec![
      "100%_done",
      "bob",
      "spec/2/spec",
      "spec/1/plan",
      "spec/1/spec"
    ]
  );
}

#[test]
fn text_matches_target_client_or_origin_case_insensitively() {
  let f = fixture();
  let search = |text: &str| {
    f.store
      .audit_table(
        &f.alice,
        AuditScope::Project("acme/demo"),
        &AuditQuery {
          text: Some(text.to_string()),
          ..query()
        },
      )
      .unwrap()
      .total
  };
  assert_eq!(search("PLAN"), 1);
  assert_eq!(search("cursor"), 1);
  assert_eq!(search("192.168"), 1);
  assert_eq!(search("10.0.0"), 3);
  assert_eq!(search("spec/"), 3);
  assert_eq!(search("nothing"), 0);
  assert_eq!(search(""), 5);
}

#[test]
fn text_wildcards_are_literal() {
  let f = fixture();
  let search = |text: &str| {
    f.store
      .audit_table(
        &f.alice,
        AuditScope::Project("acme/demo"),
        &AuditQuery {
          text: Some(text.to_string()),
          ..query()
        },
      )
      .unwrap()
      .total
  };
  assert_eq!(search("%"), 1);
  assert_eq!(search("_"), 1);
  assert_eq!(search("0%_d"), 1);
  assert_eq!(search("s_ec"), 0);
  assert_eq!(search("\\"), 0);
}

#[test]
fn filters_by_actor_client_action_and_result() {
  let f = fixture();
  let run = |q: AuditQuery| {
    f.store
      .audit_table(&f.alice, AuditScope::Project("acme/demo"), &q)
      .unwrap()
  };
  assert_eq!(
    run(AuditQuery {
      actors: vec!["bob".into()],
      ..query()
    })
    .total,
    2
  );
  assert_eq!(
    run(AuditQuery {
      actors: vec!["bob".into(), "alice".into()],
      ..query()
    })
    .total,
    5
  );
  assert_eq!(
    run(AuditQuery {
      clients: vec!["cursor".into()],
      ..query()
    })
    .total,
    1
  );
  assert_eq!(
    run(AuditQuery {
      clients: vec!["cursor".into(), "browser".into()],
      ..query()
    })
    .total,
    2
  );
  assert_eq!(
    run(AuditQuery {
      actions: vec!["member.set".into()],
      ..query()
    })
    .total,
    2
  );
  assert_eq!(
    run(AuditQuery {
      results: vec![AuditResult::Denied, AuditResult::Conflict],
      ..query()
    })
    .total,
    2
  );
  let combined = run(AuditQuery {
    actors: vec!["alice".into()],
    actions: vec!["document.write".into()],
    results: vec![AuditResult::Ok],
    ..query()
  });
  assert_eq!(targets(&combined.entries), vec!["spec/1/spec"]);
}

#[test]
fn period_limits_to_the_last_day_week_or_month() {
  let f = fixture();
  let total = |period: Period| {
    f.store
      .audit_table(
        &f.alice,
        AuditScope::Project("acme/demo"),
        &AuditQuery { period, ..query() },
      )
      .unwrap()
      .total
  };
  assert_eq!(total(Period::Day), 1);
  assert_eq!(total(Period::Week), 3);
  assert_eq!(total(Period::Month), 4);
  assert_eq!(total(Period::All), 5);
}

#[test]
fn paging_returns_pages_and_the_total() {
  let f = fixture();
  let page = |n: u32| {
    f.store
      .audit_table(
        &f.alice,
        AuditScope::Project("acme/demo"),
        &AuditQuery {
          page: n,
          page_size: 10,
          ..query()
        },
      )
      .unwrap()
  };
  assert_eq!(page(1).entries.len(), 5);
  assert_eq!(page(1).total, 5);
  assert!(page(2).entries.is_empty());
  assert_eq!(page(2).total, 5);
  assert_eq!(page(2).page, 2);

  for i in 0..30 {
    seed(
      &f.store,
      &f.alice,
      common::int(&f.store, "SELECT id FROM organizations WHERE slug = 'acme'"),
      Some(f.demo),
      row("bulk", &format!("b{i}"), None, None, "ok", 100 + i),
    );
  }
  let second = page(2);
  assert_eq!(second.total, 35);
  assert_eq!(second.entries.len(), 10);
  let first = page(1);
  assert!(
    first
      .entries
      .iter()
      .all(|e| !second.entries.iter().any(|s| s.id == e.id))
  );
  assert_eq!(page(4).entries.len(), 5);
}

#[test]
fn invalid_paging_is_a_validation_error() {
  let f = fixture();
  for q in [
    AuditQuery { page: 0, ..query() },
    AuditQuery {
      page_size: 7,
      ..query()
    },
    AuditQuery {
      page_size: 0,
      ..query()
    },
    AuditQuery {
      page_size: 1000,
      ..query()
    },
  ] {
    let error = f
      .store
      .audit_table(&f.alice, AuditScope::Project("acme/demo"), &q)
      .unwrap_err();
    assert!(matches!(error, StoreError::Validation(_)), "{error:?}");
  }
}

#[test]
fn facets_count_each_value_under_the_other_filters() {
  let f = fixture();
  let table = f
    .store
    .audit_table(&f.alice, AuditScope::Project("acme/demo"), &query())
    .unwrap();
  assert_eq!(
    facet(&table.facets.actors),
    vec![("alice".into(), 3), ("bob".into(), 2)]
  );
  assert_eq!(
    facet(&table.facets.actions),
    vec![("document.write".into(), 3), ("member.set".into(), 2)]
  );
  assert_eq!(
    facet(&table.facets.clients),
    vec![
      ("claude-code".into(), 2),
      ("browser".into(), 1),
      ("cursor".into(), 1)
    ]
  );
  assert_eq!(
    facet(&table.facets.results),
    vec![
      ("ok".into(), 3),
      ("conflict".into(), 1),
      ("denied".into(), 1)
    ]
  );
}

#[test]
fn a_facet_ignores_its_own_selection_but_honours_the_others() {
  let f = fixture();
  let table = f
    .store
    .audit_table(
      &f.alice,
      AuditScope::Project("acme/demo"),
      &AuditQuery {
        actors: vec!["bob".into()],
        actions: vec!["document.write".into()],
        ..query()
      },
    )
    .unwrap();
  assert_eq!(table.total, 1);
  assert_eq!(
    facet(&table.facets.actors),
    vec![("alice".into(), 2), ("bob".into(), 1)]
  );
  assert_eq!(
    facet(&table.facets.actions),
    vec![("document.write".into(), 1), ("member.set".into(), 1)]
  );
  assert_eq!(facet(&table.facets.clients), vec![("cursor".into(), 1)]);
  assert_eq!(facet(&table.facets.results), vec![("denied".into(), 1)]);
}

#[test]
fn facets_honour_text_period_and_ignore_paging() {
  let f = fixture();
  let table = f
    .store
    .audit_table(
      &f.alice,
      AuditScope::Project("acme/demo"),
      &AuditQuery {
        text: Some("spec/".into()),
        period: Period::Week,
        page_size: 10,
        ..query()
      },
    )
    .unwrap();
  assert_eq!(table.total, 3);
  assert_eq!(
    facet(&table.facets.actors),
    vec![("alice".into(), 2), ("bob".into(), 1)]
  );
}

#[test]
fn entries_without_a_client_are_left_out_of_the_client_facet() {
  let f = fixture();
  let table = f
    .store
    .audit_table(&f.alice, AuditScope::Project("acme/demo"), &query())
    .unwrap();
  assert_eq!(table.facets.clients.iter().map(|c| c.count).sum::<i64>(), 4);
  assert!(table.facets.clients.iter().all(|c| !c.value.is_empty()));
  assert_eq!(table.total, 5);
}

#[test]
fn access_follows_the_existing_audit_rules() {
  let f = fixture();
  let demo = AuditScope::Project("acme/demo");
  let org = AuditScope::Organization("acme");
  assert!(matches!(
    f.store.audit_table(&f.bob, demo, &query()),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.audit_table(&f.bob, org, &query()),
    Err(StoreError::Forbidden)
  ));
  f.store
    .set_member(
      &f.alice,
      "acme/demo",
      "bob",
      sdd_vault_core::model::Role::Reader,
    )
    .unwrap();
  assert!(matches!(
    f.store.audit_table(&f.bob, demo, &query()),
    Err(StoreError::Forbidden)
  ));
  f.store
    .set_member(
      &f.alice,
      "acme/demo",
      "bob",
      sdd_vault_core::model::Role::Owner,
    )
    .unwrap();
  assert!(f.store.audit_table(&f.bob, demo, &query()).is_ok());
  assert!(matches!(
    f.store.audit_table(&f.bob, org, &query()),
    Err(StoreError::Forbidden)
  ));
  assert!(f.store.audit_table(&f.ops, demo, &query()).is_ok());
  assert!(f.store.audit_table(&f.ops, org, &query()).is_ok());
  assert!(matches!(
    f.store
      .audit_table(&f.alice, AuditScope::Project("acme/missing"), &query()),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .audit_table(&f.alice, AuditScope::Organization("missing"), &query()),
    Err(StoreError::NotFound(_))
  ));
  let _ = f.other;
}
