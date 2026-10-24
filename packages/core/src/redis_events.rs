use std::time::Duration;

use redis::AsyncTypedCommands;
use redis::aio::ConnectionManager;
use tokio::sync::mpsc;

use crate::events::{EventPublisher, ProjectEvent};

pub const CHANNEL_PREFIX: &str = "sddvault:project:";
pub const QUEUE_CAPACITY: usize = 4096;
pub const RECONNECT_MIN: Duration = Duration::from_secs(1);
pub const RECONNECT_MAX: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
  #[error("invalid REDIS_URL: {0}")]
  Url(#[from] redis::RedisError),
}

/// Channel on which the events of one project travel.
pub fn channel_for(prefix: &str, project_id: i64) -> String {
  format!("{prefix}{project_id}")
}

/// Pattern matching the channels of every project.
pub fn pattern_for(prefix: &str) -> String {
  format!("{prefix}*")
}

/// Publishes events to Redis from a background task, so a slow or unreachable Redis never delays or fails a write.
#[derive(Debug, Clone)]
pub struct RedisPublisher {
  queue: mpsc::Sender<ProjectEvent>,
}

impl RedisPublisher {
  /// Starts the publishing task; must run inside a Tokio runtime.
  pub fn spawn(url: &str) -> Result<RedisPublisher, EventBusError> {
    RedisPublisher::spawn_with_prefix(url, CHANNEL_PREFIX)
  }

  pub fn spawn_with_prefix(url: &str, prefix: &str) -> Result<RedisPublisher, EventBusError> {
    let client = redis::Client::open(url)?;
    let (queue, events) = mpsc::channel(QUEUE_CAPACITY);
    tokio::spawn(run(client, prefix.to_string(), events));
    Ok(RedisPublisher { queue })
  }
}

async fn connect(client: &redis::Client) -> ConnectionManager {
  let mut delay = RECONNECT_MIN;
  loop {
    match ConnectionManager::new(client.clone()).await {
      Ok(manager) => return manager,
      Err(error) => {
        tracing::warn!(%error, "event bus unreachable, retrying");
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(RECONNECT_MAX);
      }
    }
  }
}

async fn run(client: redis::Client, prefix: String, mut events: mpsc::Receiver<ProjectEvent>) {
  let mut manager = connect(&client).await;
  while let Some(event) = events.recv().await {
    let payload = match serde_json::to_string(&event) {
      Ok(payload) => payload,
      Err(error) => {
        tracing::error!(%error, "could not encode a project event");
        continue;
      }
    };
    if let Err(error) = manager
      .publish(channel_for(&prefix, event.project_id), payload)
      .await
    {
      tracing::warn!(%error, audit_id = event.audit_id, "could not publish a project event");
    }
  }
}

impl EventPublisher for RedisPublisher {
  fn publish(&self, event: &ProjectEvent) {
    if let Err(error) = self.queue.try_send(event.clone()) {
      tracing::warn!(%error, audit_id = event.audit_id, "dropped a project event");
    }
  }
}
