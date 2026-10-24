mod common;

use std::time::{Duration, Instant};

use sdd_vault_core::model::AuditResult;
use sdd_vault_core::store::views::audit_table::{AuditQuery, AuditScope, Period, SortOrder};

const ENTRIES: i64 = 100_000;
const ACTORS: i64 = 20;
const SAMPLES: usize = 60;
const P95_LIMIT: Duration = Duration::from_millis(500);

fn percentile(mut samples: Vec<Duration>, p: f64) -> Duration {
  samples.sort();
  let index = ((samples.len() as f64 - 1.0) * p).round() as usize;
  samples[index]
}

fn queries() -> Vec<AuditQuery> {
  let base = AuditQuery {
    page_size: 50,
    ..AuditQuery::default()
  };
  vec![
    base.clone(),
    AuditQuery {
      sort: SortOrder::Oldest,
      page: 3,
      ..base.clone()
    },
    AuditQuery {
      text: Some("spec/12".into()),
      ..base.clone()
    },
    AuditQuery {
      actors: vec!["actor3".into(), "actor7".into()],
      results: vec![AuditResult::Denied],
      ..base.clone()
    },
    AuditQuery {
      actions: vec!["document.write".into()],
      clients: vec!["cursor".into()],
      period: Period::Month,
      ..base
    },
  ]
}

#[test]
#[ignore = "performance check, run with --release and --ignored"]
fn p95_table_page_with_facets_over_100000_entries_under_500ms() {
  let test_store = common::test_store_with_pool(4);
  let store = test_store.store.clone();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  for n in 0..ACTORS {
    common::person(&store, &format!("actor{n}"));
  }
  common::exec(
    &store,
    &format!(
      "INSERT INTO audit_log (actor_id, organization_id, project_id, action, target, client, origin, result, created_at)
       SELECT (SELECT min(id) FROM users WHERE handle LIKE 'actor%') + (g % {ACTORS}),
              p.organization_id, p.id,
              (ARRAY['document.write', 'member.set', 'spec.create', 'project.archive', 'task.update'])[1 + g % 5],
              'spec/' || (g % 40) || '/plan',
              (ARRAY['claude-code', 'cursor', 'browser', 'codex'])[1 + g % 4],
              '10.0.' || (g % 255) || '.' || (g % 200),
              (ARRAY['ok', 'ok', 'ok', 'denied', 'conflict'])[1 + g % 5],
              now() - (g || ' minutes')::interval
         FROM generate_series(1, {ENTRIES}) g, projects p WHERE p.slug = 'demo'"
    ),
  );
  common::exec(&store, "ANALYZE audit_log");
  assert_eq!(
    common::int(
      &store,
      "SELECT count(*) FROM audit_log WHERE project_id IS NOT NULL"
    ),
    ENTRIES + 1
  );

  let queries = queries();
  store
    .audit_table(&root, AuditScope::Project("acme/demo"), &queries[0])
    .unwrap();
  let mut samples = Vec::with_capacity(SAMPLES);
  for index in 0..SAMPLES {
    let query = &queries[index % queries.len()];
    let started = Instant::now();
    let table = store
      .audit_table(&root, AuditScope::Project("acme/demo"), query)
      .unwrap();
    samples.push(started.elapsed());
    assert!(table.total > 0);
    assert!(!table.facets.actors.is_empty());
  }
  let p95 = percentile(samples.clone(), 0.95);
  let p50 = percentile(samples, 0.5);
  println!("audit table over {ENTRIES} entries: p50 {p50:?}, p95 {p95:?}");
  assert!(p95 < P95_LIMIT, "p95 {p95:?} over {P95_LIMIT:?}");
}
