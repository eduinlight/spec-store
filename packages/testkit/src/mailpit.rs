use std::time::{Duration, Instant};

use serde_json::Value;

pub fn api_url() -> String {
  std::env::var("MAILPIT_URL").unwrap_or_else(|_| {
    let port = std::env::var("MAILPIT_UI_PORT").unwrap_or_else(|_| "58025".into());
    format!("http://localhost:{port}")
  })
}

pub fn smtp_host() -> String {
  std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into())
}

pub fn smtp_port() -> u16 {
  std::env::var("SMTP_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(51025)
}

#[derive(Debug, Clone)]
pub struct Message {
  pub id: String,
  pub subject: String,
  pub to: Vec<String>,
  pub text: String,
  pub html: String,
}

pub fn unique_address(label: &str) -> String {
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  format!("{label}-{}-{nanos}@example.test", std::process::id())
}

pub async fn messages_to(address: &str) -> Vec<Message> {
  let http = reqwest::Client::new();
  let search: Value = http
    .get(format!("{}/api/v1/search", api_url()))
    .query(&[("query", format!("to:\"{address}\""))])
    .send()
    .await
    .expect("mailpit search")
    .json()
    .await
    .expect("mailpit json");
  let mut found = Vec::new();
  for summary in search["messages"].as_array().cloned().unwrap_or_default() {
    let id = summary["ID"].as_str().unwrap_or_default().to_string();
    let full: Value = http
      .get(format!("{}/api/v1/message/{id}", api_url()))
      .send()
      .await
      .expect("mailpit message")
      .json()
      .await
      .expect("mailpit message json");
    found.push(Message {
      id,
      subject: full["Subject"].as_str().unwrap_or_default().to_string(),
      to: full["To"]
        .as_array()
        .map(|a| {
          a.iter()
            .filter_map(|t| t["Address"].as_str().map(str::to_string))
            .collect()
        })
        .unwrap_or_default(),
      text: full["Text"].as_str().unwrap_or_default().to_string(),
      html: full["HTML"].as_str().unwrap_or_default().to_string(),
    });
  }
  found
}

pub async fn wait_for_messages(address: &str, count: usize, timeout: Duration) -> Vec<Message> {
  let started = Instant::now();
  loop {
    let messages = messages_to(address).await;
    if messages.len() >= count || started.elapsed() > timeout {
      return messages;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
  }
}
