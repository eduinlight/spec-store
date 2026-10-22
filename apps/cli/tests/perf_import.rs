mod common;

use std::fs;
use std::time::{Duration, Instant};

const SPECS: usize = 100;
const KINDS: [&str; 5] = ["spec", "plan", "tasks", "research", "validation"];
const DOCUMENT_BYTES: usize = 20 * 1024;
const LIMIT: Duration = Duration::from_secs(30);

#[tokio::test(flavor = "multi_thread")]
#[ignore = "performance check, run in release by make test"]
async fn importing_one_hundred_specs_through_the_cli_takes_under_30_seconds() {
  let env = common::start("perf-import").await;
  let alice = env.person("alice");
  env
    .store
    .create_organization(&alice, "acme", "Acme")
    .unwrap();
  env
    .store
    .create_project(&alice, "acme/demo", "Demo")
    .unwrap();

  let root = common::temp_dir("perf-import-tree");
  let line = "- [ ] T01 (FR-1) Something to do with a path — Done when: it is checked\n";
  let body = line.repeat(DOCUMENT_BYTES / line.len() + 1);
  for n in 1..=SPECS {
    let folder = root.join(format!("specs/{n:03}-spec-{n}"));
    fs::create_dir_all(&folder).unwrap();
    for kind in KINDS {
      fs::write(folder.join(format!("{kind}.md")), &body).unwrap();
    }
  }
  env.use_identity("alice");

  let started = Instant::now();
  let out = env
    .run(&["import", "acme/demo", root.to_str().unwrap()])
    .await;
  let elapsed = started.elapsed();
  let stdout = common::text(&out.stdout);
  assert!(
    out.status.success(),
    "stdout: {stdout}\nstderr: {}",
    common::text(&out.stderr)
  );
  assert!(
    stdout.contains(&format!(
      "imported {SPECS} specs and {} documents",
      SPECS * KINDS.len()
    )),
    "{stdout}"
  );
  let caller = env.store.caller_by_handle("alice", false).unwrap();
  assert_eq!(
    env.store.list_specs(&caller, "acme/demo").unwrap().len(),
    SPECS
  );
  let _ = fs::remove_dir_all(&root);
  eprintln!("imported {SPECS} specs through the CLI in {elapsed:?}");
  assert!(elapsed < LIMIT, "import took {elapsed:?}");
}
