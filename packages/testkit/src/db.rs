use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub const TEST_PREFIX: &str = "sdd_vault_test_";
pub const TEMPLATE_PREFIX: &str = "sdd_vault_tpl_";

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static TEMPLATE: OnceLock<String> = OnceLock::new();
static CLONE_LOCK: Mutex<()> = Mutex::new(());

pub fn database_url() -> String {
  std::env::var("DATABASE_URL")
    .expect("DATABASE_URL must point at the compose PostgreSQL; run the suite with `make test`")
}

pub fn with_database(url: &str, database: &str) -> String {
  let (base, query) = match url.split_once('?') {
    Some((base, query)) => (base, Some(query)),
    None => (url, None),
  };
  let prefix = base
    .rsplit_once('/')
    .map(|(prefix, _)| prefix)
    .expect("database URL has a path");
  match query {
    Some(query) => format!("{prefix}/{database}?{query}"),
    None => format!("{prefix}/{database}"),
  }
}

fn admin_connection() -> PgConnection {
  let url = database_url();
  PgConnection::establish(&url).unwrap_or_else(|e| panic!("cannot connect to {url}: {e}"))
}

fn create_template(migrate: fn(&str)) -> String {
  let name = format!("{TEMPLATE_PREFIX}{}", std::process::id());
  let mut admin = admin_connection();
  admin
    .batch_execute(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
    .expect("drop stale template database");
  admin
    .batch_execute(&format!("CREATE DATABASE {name}"))
    .expect("create template database");
  migrate(&with_database(&database_url(), &name));
  name
}

pub struct TestDb {
  pub name: String,
  pub url: String,
}

impl TestDb {
  pub fn new(migrate: fn(&str)) -> TestDb {
    let template = TEMPLATE.get_or_init(|| create_template(migrate));
    let name = format!(
      "{TEST_PREFIX}{}_{}",
      std::process::id(),
      COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    {
      let _guard = CLONE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
      admin_connection()
        .batch_execute(&format!("CREATE DATABASE {name} TEMPLATE {template}"))
        .expect("clone template database");
    }
    let url = with_database(&database_url(), &name);
    TestDb { name, url }
  }

  pub fn empty() -> TestDb {
    let name = format!(
      "{TEST_PREFIX}{}_{}",
      std::process::id(),
      COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    admin_connection()
      .batch_execute(&format!("CREATE DATABASE {name}"))
      .expect("create empty database");
    let url = with_database(&database_url(), &name);
    TestDb { name, url }
  }

  pub fn connect(&self) -> PgConnection {
    PgConnection::establish(&self.url).expect("connect to test database")
  }
}

impl Drop for TestDb {
  fn drop(&mut self) {
    let _ = admin_connection().batch_execute(&format!(
      "DROP DATABASE IF EXISTS {} WITH (FORCE)",
      self.name
    ));
  }
}

pub fn database_exists(name: &str) -> bool {
  #[derive(QueryableByName)]
  struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
  }
  let row: Count =
    diesel::sql_query("SELECT count(*) AS count FROM pg_database WHERE datname = $1")
      .bind::<diesel::sql_types::Text, _>(name)
      .get_result(&mut admin_connection())
      .expect("query pg_database");
  row.count > 0
}
