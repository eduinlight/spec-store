use std::collections::HashMap;
use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::http::header::ORIGIN;
use axum::response::Response;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use redis::aio::PubSub;
use sdd_vault_core::address::ProjectRef;
use sdd_vault_core::events::{InProcessPublisher, ProjectEvent};
use sdd_vault_core::identity::IdentityError;
use sdd_vault_core::model::{Caller, StoreError};
use sdd_vault_core::redis_events::{
  CHANNEL_PREFIX, EventBusError, RECONNECT_MAX, RECONNECT_MIN, pattern_for,
};
use sdd_vault_core::store::projects::access_in;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{broadcast, watch};

use crate::auth::{Principal, bearer_in};
use crate::cookies::{SESSION_COOKIE, cookie};
use crate::error::ApiError;
use crate::state::AppState;

pub const HUB_CAPACITY: usize = 1024;
pub const MAX_SUBSCRIPTIONS: usize = 32;
pub const MAX_MESSAGE_BYTES: usize = 4096;
pub const SEND_TIMEOUT: Duration = Duration::from_secs(5);
pub const CLOSE_TRY_AGAIN: u16 = 1013;
pub const CLOSE_POLICY: u16 = 1008;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveSettings {
  pub ping_interval: Duration,
  pub recheck_interval: Duration,
}

impl Default for LiveSettings {
  fn default() -> Self {
    LiveSettings {
      ping_interval: Duration::from_secs(20),
      recheck_interval: Duration::from_secs(30),
    }
  }
}

/// One per API instance: receives every project event of the platform once and fans it out to the sockets of the instance.
#[derive(Clone)]
pub struct LiveHub {
  sender: broadcast::Sender<ProjectEvent>,
  ready: watch::Receiver<bool>,
}

impl LiveHub {
  /// A hub that never receives events.
  pub fn detached() -> LiveHub {
    let (sender, _) = broadcast::channel(HUB_CAPACITY);
    let (_, ready) = watch::channel(true);
    LiveHub { sender, ready }
  }

  /// A hub fed by the writes of the same process.
  pub fn in_process(bus: &InProcessPublisher) -> LiveHub {
    let (_, ready) = watch::channel(true);
    LiveHub {
      sender: bus.sender(),
      ready,
    }
  }

  /// A hub with a single pattern subscription to Redis; must run inside a Tokio runtime.
  pub fn redis(url: &str) -> Result<LiveHub, EventBusError> {
    LiveHub::redis_with_prefix(url, CHANNEL_PREFIX)
  }

  pub fn redis_with_prefix(url: &str, prefix: &str) -> Result<LiveHub, EventBusError> {
    let client = redis::Client::open(url)?;
    let (sender, _) = broadcast::channel(HUB_CAPACITY);
    let (ready_tx, ready) = watch::channel(false);
    tokio::spawn(subscription(
      client,
      pattern_for(prefix),
      sender.clone(),
      ready_tx,
    ));
    Ok(LiveHub { sender, ready })
  }

  pub fn subscribe(&self) -> broadcast::Receiver<ProjectEvent> {
    self.sender.subscribe()
  }

  /// Waits until the hub receives events; true when it does within the timeout.
  pub async fn wait_ready(&self, timeout: Duration) -> bool {
    let mut ready = self.ready.clone();
    tokio::time::timeout(timeout, ready.wait_for(|connected| *connected))
      .await
      .is_ok_and(|result| result.is_ok())
  }
}

async fn subscription(
  client: redis::Client,
  pattern: String,
  sender: broadcast::Sender<ProjectEvent>,
  ready: watch::Sender<bool>,
) {
  let mut delay = RECONNECT_MIN;
  loop {
    match open(&client, &pattern).await {
      Ok(pubsub) => {
        delay = RECONNECT_MIN;
        let _ = ready.send(true);
        forward(pubsub, &sender).await;
        let _ = ready.send(false);
        tracing::warn!("event bus subscription lost, reconnecting");
      }
      Err(error) => tracing::warn!(%error, "event bus unreachable, retrying"),
    }
    tokio::time::sleep(delay).await;
    delay = (delay * 2).min(RECONNECT_MAX);
  }
}

async fn open(client: &redis::Client, pattern: &str) -> redis::RedisResult<PubSub> {
  let mut pubsub = client.get_async_pubsub().await?;
  pubsub.psubscribe(pattern).await?;
  Ok(pubsub)
}

async fn forward(mut pubsub: PubSub, sender: &broadcast::Sender<ProjectEvent>) {
  let mut messages = pubsub.on_message();
  while let Some(message) = messages.next().await {
    let payload: String = match message.get_payload() {
      Ok(payload) => payload,
      Err(error) => {
        tracing::warn!(%error, "unreadable event bus message");
        continue;
      }
    };
    match serde_json::from_str::<ProjectEvent>(&payload) {
      Ok(event) => {
        let _ = sender.send(event);
      }
      Err(error) => tracing::warn!(%error, "malformed event bus message"),
    }
  }
}

#[derive(Debug, Clone)]
enum Credential {
  Session(String),
  Bearer(String),
}

impl Credential {
  fn of(headers: &HeaderMap) -> Option<Credential> {
    if let Some(token) = bearer_in(headers) {
      return Some(Credential::Bearer(token.to_string()));
    }
    cookie(headers, SESSION_COOKIE).map(Credential::Session)
  }

  async fn valid(&self, state: &AppState) -> bool {
    match self {
      Credential::Session(secret) => {
        let secret = secret.clone();
        state
          .blocking(move |store| store.session(&secret))
          .await
          .is_ok()
      }
      Credential::Bearer(token) => {
        match state.verifier.verify(token, &state.config.audience).await {
          Ok(_) => true,
          Err(IdentityError::Unavailable(_)) => true,
          Err(_) => false,
        }
      }
    }
  }
}

fn origin_allowed(state: &AppState, headers: &HeaderMap) -> bool {
  let expected = state.config.app_url.trim_end_matches('/');
  headers
    .get(ORIGIN)
    .and_then(|value| value.to_str().ok())
    .is_some_and(|origin| origin.trim_end_matches('/') == expected)
}

/// WebSocket upgrade authenticated by the browser session cookie or a bearer token.
pub async fn socket(
  State(state): State<AppState>,
  principal: Principal,
  headers: HeaderMap,
  upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
  if principal.via_session && !origin_allowed(&state, &headers) {
    return Err(ApiError::Store(StoreError::Forbidden));
  }
  let credential = Credential::of(&headers).ok_or(ApiError::Unauthenticated)?;
  Ok(
    upgrade
      .max_message_size(MAX_MESSAGE_BYTES)
      .max_frame_size(MAX_MESSAGE_BYTES)
      .on_upgrade(move |socket| session(state, principal.caller, credential, socket)),
  )
}

#[derive(Debug, Deserialize)]
struct Command {
  subscribe: Option<String>,
  unsubscribe: Option<String>,
}

type Sink = SplitSink<WebSocket, Message>;
type Subscriptions = HashMap<i64, String>;

async fn deliver(sink: &mut Sink, message: Message) -> bool {
  matches!(
    tokio::time::timeout(SEND_TIMEOUT, sink.send(message)).await,
    Ok(Ok(()))
  )
}

async fn reply(sink: &mut Sink, value: Value) -> bool {
  deliver(sink, Message::text(value.to_string())).await
}

async fn readable_project(state: &AppState, caller: &Caller, address: &str) -> Result<i64, ()> {
  let target = match ProjectRef::parse(address) {
    Ok(parsed) => format!("{}/{}", parsed.organization, parsed.project),
    Err(_) => return Err(()),
  };
  let caller = caller.clone();
  let found = state
    .blocking(move |store| {
      store.with_conn(|conn| access_in(conn, &caller, &target).map(|access| access.project_id))
    })
    .await;
  match found {
    Ok(project_id) => Ok(project_id),
    Err(ApiError::Store(StoreError::NotFound(_))) => Err(()),
    Err(error) => {
      tracing::warn!(%error, "could not authorize a live subscription");
      Err(())
    }
  }
}

async fn handle(
  state: &AppState,
  caller: &Caller,
  subscriptions: &mut Subscriptions,
  text: &str,
) -> Value {
  let Ok(command) = serde_json::from_str::<Command>(text) else {
    return json!({"error": "invalid_message"});
  };
  if let Some(project) = command.subscribe {
    if subscriptions.len() >= MAX_SUBSCRIPTIONS
      && !subscriptions.values().any(|known| *known == project)
    {
      return json!({"error": "too_many_subscriptions", "project": project});
    }
    return match readable_project(state, caller, &project).await {
      Ok(project_id) => {
        subscriptions.insert(project_id, project.clone());
        json!({"subscribed": project})
      }
      Err(()) => json!({"error": "not_found", "project": project}),
    };
  }
  if let Some(project) = command.unsubscribe {
    subscriptions.retain(|_, known| *known != project);
    return json!({"unsubscribed": project});
  }
  json!({"error": "invalid_message"})
}

async fn recheck(
  state: &AppState,
  caller: &Caller,
  credential: &Credential,
  subscriptions: &mut Subscriptions,
  sink: &mut Sink,
) -> bool {
  if !credential.valid(state).await {
    return false;
  }
  let known: Vec<(i64, String)> = subscriptions
    .iter()
    .map(|(id, address)| (*id, address.clone()))
    .collect();
  for (project_id, address) in known {
    if readable_project(state, caller, &address).await != Ok(project_id) {
      subscriptions.remove(&project_id);
      if !reply(sink, json!({"error": "not_found", "project": address})).await {
        return false;
      }
    }
  }
  true
}

async fn session(state: AppState, caller: Caller, credential: Credential, socket: WebSocket) {
  let settings = state.live_settings;
  let (mut sink, mut incoming) = socket.split();
  let mut events = state.live.subscribe();
  let mut subscriptions = Subscriptions::new();
  let mut ping = tokio::time::interval_at(
    tokio::time::Instant::now() + settings.ping_interval,
    settings.ping_interval,
  );
  let mut checks = tokio::time::interval_at(
    tokio::time::Instant::now() + settings.recheck_interval,
    settings.recheck_interval,
  );
  let mut alive = true;
  let mut close_code = None;
  loop {
    tokio::select! {
      frame = incoming.next() => match frame {
        Some(Ok(Message::Text(text))) => {
          alive = true;
          let answer = handle(&state, &caller, &mut subscriptions, text.as_str()).await;
          if !reply(&mut sink, answer).await {
            return;
          }
        }
        Some(Ok(Message::Binary(_))) => {
          alive = true;
          if !reply(&mut sink, json!({"error": "invalid_message"})).await {
            return;
          }
        }
        Some(Ok(Message::Ping(_) | Message::Pong(_))) => alive = true,
        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
      },
      received = events.recv() => match received {
        Ok(event) => {
          if let Some(project) = subscriptions.get(&event.project_id) {
            let message = json!({
              "project": project,
              "audit_id": event.audit_id,
              "action": event.action,
              "result": event.result.as_str(),
            });
            if !reply(&mut sink, message).await {
              return;
            }
          }
        }
        Err(broadcast::error::RecvError::Lagged(_)) => {
          close_code = Some(CLOSE_TRY_AGAIN);
          break;
        }
        Err(broadcast::error::RecvError::Closed) => break,
      },
      _ = ping.tick() => {
        if !alive {
          break;
        }
        alive = false;
        if !deliver(&mut sink, Message::Ping(Default::default())).await {
          return;
        }
      }
      _ = checks.tick() => {
        if !recheck(&state, &caller, &credential, &mut subscriptions, &mut sink).await {
          close_code = Some(CLOSE_POLICY);
          break;
        }
      }
    }
  }
  let frame = close_code.map(|code| CloseFrame {
    code,
    reason: "".into(),
  });
  let _ = deliver(&mut sink, Message::Close(frame)).await;
}
