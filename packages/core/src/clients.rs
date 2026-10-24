use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::keycloak::KeycloakAdmin;

pub const CLI_CLIENT_ID: &str = "sdd-vault-cli";
pub const CLI_CLIENT_NAME: &str = "sddvault CLI";
pub const WEB_APP_CLIENT_NAME: &str = "Web app";
pub const API_CLIENT_FALLBACK: &str = "API client";
pub const MCP_CLIENT_FALLBACK: &str = "MCP client";
pub const NAME_TTL: Duration = Duration::from_secs(600);

pub type NameFuture<'a> = Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;
pub trait ClientNameResolver: Send + Sync {
  fn name<'a>(&'a self, user_subject: &'a str, client_id: &'a str) -> NameFuture<'a>;
}

pub struct KeycloakConsentNames {
  admin: Arc<KeycloakAdmin>,
}

impl KeycloakConsentNames {
  pub fn new(admin: Arc<KeycloakAdmin>) -> Self {
    KeycloakConsentNames { admin }
  }
}

impl ClientNameResolver for KeycloakConsentNames {
  fn name<'a>(&'a self, user_subject: &'a str, client_id: &'a str) -> NameFuture<'a> {
    Box::pin(async move {
      match self.admin.list_consents(user_subject).await {
        Ok(consents) => consents
          .into_iter()
          .find(|consent| consent.client_id == client_id)
          .and_then(|consent| consent.name)
          .map(|name| name.trim().to_string())
          .filter(|name| !name.is_empty()),
        Err(error) => {
          tracing::warn!(%error, client_id, "could not resolve the name of a client");
          None
        }
      }
    })
  }
}

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;
type CacheKey = (String, String);
pub struct CachedResolver {
  inner: Arc<dyn ClientNameResolver>,
  ttl: Duration,
  clock: Clock,
  entries: Mutex<HashMap<CacheKey, (Instant, Option<String>)>>,
}

impl CachedResolver {
  pub fn new(inner: Arc<dyn ClientNameResolver>) -> Self {
    CachedResolver::with_clock(inner, NAME_TTL, Arc::new(Instant::now))
  }

  pub fn with_clock(inner: Arc<dyn ClientNameResolver>, ttl: Duration, clock: Clock) -> Self {
    CachedResolver {
      inner,
      ttl,
      clock,
      entries: Mutex::new(HashMap::new()),
    }
  }

  fn cached(&self, key: &CacheKey) -> Option<Option<String>> {
    let now = (self.clock)();
    let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
    entries
      .get(key)
      .filter(|(stored_at, _)| now.saturating_duration_since(*stored_at) < self.ttl)
      .map(|(_, name)| name.clone())
  }
}

impl ClientNameResolver for CachedResolver {
  fn name<'a>(&'a self, user_subject: &'a str, client_id: &'a str) -> NameFuture<'a> {
    Box::pin(async move {
      let key = (user_subject.to_string(), client_id.to_string());
      if let Some(name) = self.cached(&key) {
        return name;
      }
      let name = self.inner.name(user_subject, client_id).await;
      let now = (self.clock)();
      self
        .entries
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, (now, name.clone()));
      name
    })
  }
}

pub async fn resolved_name(
  resolver: Option<&dyn ClientNameResolver>,
  user_subject: &str,
  client_id: Option<&str>,
  fallback: &str,
) -> String {
  let (Some(resolver), Some(client_id)) = (resolver, client_id) else {
    return fallback.to_string();
  };
  resolver
    .name(user_subject, client_id)
    .await
    .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use super::*;

  struct Counting {
    calls: AtomicUsize,
    answer: Option<String>,
  }

  impl ClientNameResolver for Counting {
    fn name<'a>(&'a self, _: &'a str, _: &'a str) -> NameFuture<'a> {
      Box::pin(async move {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.answer.clone()
      })
    }
  }

  fn counting(answer: Option<&str>) -> Arc<Counting> {
    Arc::new(Counting {
      calls: AtomicUsize::new(0),
      answer: answer.map(str::to_string),
    })
  }

  fn manual_clock() -> (Clock, Arc<Mutex<Instant>>) {
    let now = Arc::new(Mutex::new(Instant::now()));
    let reader = now.clone();
    (Arc::new(move || *reader.lock().unwrap()), now)
  }

  #[tokio::test]
  async fn a_name_is_looked_up_once_per_user_and_client_within_the_ttl() {
    let inner = counting(Some("Cursor"));
    let (clock, now) = manual_clock();
    let cached = CachedResolver::with_clock(inner.clone(), Duration::from_secs(600), clock);
    assert_eq!(cached.name("u1", "c1").await.as_deref(), Some("Cursor"));
    assert_eq!(cached.name("u1", "c1").await.as_deref(), Some("Cursor"));
    *now.lock().unwrap() += Duration::from_secs(599);
    assert_eq!(cached.name("u1", "c1").await.as_deref(), Some("Cursor"));
    assert_eq!(inner.calls.load(Ordering::SeqCst), 1);
    assert_eq!(cached.name("u2", "c1").await.as_deref(), Some("Cursor"));
    assert_eq!(cached.name("u1", "c2").await.as_deref(), Some("Cursor"));
    assert_eq!(inner.calls.load(Ordering::SeqCst), 3);
  }

  #[tokio::test]
  async fn the_name_is_looked_up_again_after_the_ttl() {
    let inner = counting(Some("Cursor"));
    let (clock, now) = manual_clock();
    let cached = CachedResolver::with_clock(inner.clone(), Duration::from_secs(600), clock);
    cached.name("u1", "c1").await;
    *now.lock().unwrap() += Duration::from_secs(601);
    cached.name("u1", "c1").await;
    assert_eq!(inner.calls.load(Ordering::SeqCst), 2);
  }

  #[tokio::test]
  async fn unresolved_names_are_cached_too() {
    let inner = counting(None);
    let cached = CachedResolver::new(inner.clone());
    assert_eq!(cached.name("u1", "c1").await, None);
    assert_eq!(cached.name("u1", "c1").await, None);
    assert_eq!(inner.calls.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn the_fallback_names_a_client_nobody_could_resolve() {
    let known = counting(Some("Zed"));
    let unknown = counting(None);
    assert_eq!(
      resolved_name(Some(&*known), "u", Some("c"), "MCP client").await,
      "Zed"
    );
    assert_eq!(
      resolved_name(Some(&*unknown), "u", Some("c"), "MCP client").await,
      "MCP client"
    );
    assert_eq!(
      resolved_name(Some(&*known), "u", None, "MCP client").await,
      "MCP client"
    );
    assert_eq!(
      resolved_name(None, "u", Some("c"), "API client").await,
      "API client"
    );
  }
}
