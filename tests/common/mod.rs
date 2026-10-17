use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[allow(dead_code)]
pub fn temp_db_path(label: &str) -> PathBuf {
  let n = COUNTER.fetch_add(1, Ordering::SeqCst);
  let dir = std::env::temp_dir().join("spec-store-tests");
  std::fs::create_dir_all(&dir).expect("temp dir");
  dir.join(format!("{label}-{}-{n}.db", std::process::id()))
}

#[allow(dead_code)]
pub fn temp_dir(label: &str) -> PathBuf {
  let n = COUNTER.fetch_add(1, Ordering::SeqCst);
  let dir = std::env::temp_dir()
    .join("spec-store-tests")
    .join(format!("{label}-{}-{n}", std::process::id()));
  if dir.exists() {
    std::fs::remove_dir_all(&dir).expect("clear temp dir");
  }
  std::fs::create_dir_all(&dir).expect("temp dir");
  dir
}
