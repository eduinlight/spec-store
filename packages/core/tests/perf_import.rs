mod common;

use std::fs;
use std::time::{Duration, Instant};

use spec_store::model::Role;
use spec_store::store::Store;
use spec_store::store::transfer::ImportOptions;
use spec_store::tree::scan;

const SPECS: usize = 100;
const DOCUMENT_BYTES: usize = 20 * 1024;
const LIMIT: Duration = Duration::from_secs(10);

#[test]
#[ignore]
fn importing_one_hundred_specs_takes_under_ten_seconds() {
  let root = common::temp_dir("perf-import");
  let line = "- [ ] T01 (FR-1) Something to do with a path — Done when: it is checked\n";
  let body: String = line.repeat(DOCUMENT_BYTES / line.len() + 1);
  for n in 1..=SPECS {
    let folder = root.join(format!("specs/{n:03}-spec-{n}"));
    fs::create_dir_all(&folder).unwrap();
    for kind in ["spec", "plan", "tasks", "research", "validation"] {
      fs::write(folder.join(format!("{kind}.md")), &body).unwrap();
    }
  }

  let store = Store::open(common::temp_db_path("perf-import")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  store.create_project(&admin, "demo", "Demo").unwrap();
  store
    .set_member(&admin, "demo", "root", Role::Owner)
    .unwrap();

  let started = Instant::now();
  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  let report = store
    .import_tree(&admin, "demo", &tree, &ImportOptions::default())
    .unwrap();
  let elapsed = started.elapsed();
  assert_eq!(report.specs.len(), SPECS);
  assert_eq!(report.documents, SPECS * 5);
  println!("imported {SPECS} specs in {elapsed:?}");
  assert!(elapsed < LIMIT, "import took {elapsed:?}");
}
