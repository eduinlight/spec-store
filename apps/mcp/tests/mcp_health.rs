mod common;

#[tokio::test]
async fn liveness_and_readiness_answer() {
  let mcp = common::start().await;
  let http = reqwest::Client::new();
  let live = http
    .get(format!("{}/healthz", mcp.base))
    .send()
    .await
    .unwrap();
  assert_eq!(live.status(), 200);
  assert_eq!(live.text().await.unwrap(), "ok");
  let ready = http
    .get(format!("{}/readyz", mcp.base))
    .send()
    .await
    .unwrap();
  assert_eq!(ready.status(), 200);
}

#[tokio::test]
async fn readiness_fails_when_the_database_is_unreachable() {
  let mut mcp = common::start().await;
  let db = std::mem::replace(&mut mcp.db, sdd_vault_testkit::TestDb::empty());
  drop(db);
  let ready = reqwest::get(format!("{}/readyz", mcp.base)).await.unwrap();
  assert_eq!(ready.status(), 503);
  let body: serde_json::Value = ready.json().await.unwrap();
  assert_eq!(body["database"], "unavailable");
}
