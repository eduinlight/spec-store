mod common;

use std::fs;
use std::time::{Duration, Instant};

use spec_store_core::store::Store;
use spec_store_core::store::transfer::ImportOptions;
use spec_store_core::tree::scan;

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

  let test_store = common::test_store();
  let store: &Store = &test_store;
  let admin = common::person(store, "root");
  common::organization(store, &admin, common::ORG);
  store.create_project(&admin, "acme/demo", "Demo").unwrap();

  let started = Instant::now();
  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  let report = store
    .import_tree(&admin, "acme/demo", &tree, &ImportOptions::default())
    .unwrap();
  let elapsed = started.elapsed();
  assert_eq!(report.specs.len(), SPECS);
  assert_eq!(report.documents, SPECS * 5);
  println!("imported {SPECS} specs in {elapsed:?}");
  assert!(elapsed < LIMIT, "import took {elapsed:?}");
}
