# Research — Spec 014

Spec: `specs/014-overview-activity-live/spec.md` · Constitution: v3.0 · Date: 2026-10-24

Versions were read from crates.io on 2026-10-24 and the behaviours below were checked against the crate sources of those versions and against the `redis:8-alpine` container of the compose stack.

## R1 — Redis client: `redis` 1.8.0 with `tokio-comp` and `connection-manager`

- Question: which client publishes events and holds the single pattern subscription per instance?
- Finding: `redis` (redis-rs) is the Redis client the Redis project lists for Rust. `aio::PubSub` (from `Client::get_async_pubsub`) supports `psubscribe` and exposes `on_message()` as a `Stream`; the stream ends when the connection drops, which is the signal to reconnect. `aio::ConnectionManager` (feature `connection-manager`) is a cloneable multiplexed connection that reconnects by itself, which suits the publisher. A subscription connection cannot run other commands, so the hub uses its own `PubSub` and the publisher its own `ConnectionManager`. `default-features = false` drops `acl`, `streams`, `geospatial`, `script` and `num-bigint`, none of which is used.
- Source: https://docs.rs/redis/1.8.0/redis/aio/struct.PubSub.html ; https://docs.rs/redis/1.8.0/redis/aio/struct.ConnectionManager.html ; https://docs.rs/redis/1.8.0/redis/#async ; https://redis.io/docs/latest/develop/pubsub/ ; https://redis.io/docs/latest/commands/psubscribe/
- Decision: `redis = { version = "1.8", default-features = false, features = ["tokio-comp", "connection-manager"] }` in `[workspace.dependencies]`. Rejected: `deadpool-redis` or `bb8-redis` (a pool of connections for request/response traffic; publishing is one fire-and-forget command per event and the manager already multiplexes), hand-written RESP over a `TcpStream` (reimplements reconnection and parsing).

## R2 — Redis pub/sub semantics: notifications, not a log

- Question: is Redis pub/sub enough for "something changed, refetch"?
- Finding: pub/sub delivers at most once and keeps nothing; a subscriber that is disconnected misses what was published meanwhile. `PSUBSCRIBE` with a glob pattern receives every matching channel over one connection, so one subscription per instance covers all projects. The pages refetch their data from PostgreSQL when an event arrives and again whenever their WebSocket reconnects, so a lost notification only delays an update (NFR-2) and never loses data.
- Source: https://redis.io/docs/latest/develop/pubsub/ ; https://redis.io/docs/latest/commands/psubscribe/ ; https://redis.io/docs/latest/commands/publish/
- Decision: channel `sddvault:project:{project_id}`, pattern `sddvault:project:*`, JSON payload `{project_id, organization_id, project_address, audit_id, action, result}` without document content. Publishing runs in a background task fed by a bounded queue so a slow or unreachable Redis never delays or fails a write. The `RedisPublisher` lives in `packages/core` (module `redis_events`) beside the other adapters because both the API and the MCP server publish; the subscriber (`LiveHub`) lives in `apps/api`. Rejected: Redis Streams (durable replay is not needed and would add consumer-group state), PostgreSQL `LISTEN/NOTIFY` (see the plan).

## R3 — WebSockets: axum feature `ws`

- Question: how does the API upgrade connections and authenticate them?
- Finding: axum's `ws` feature (tokio-tungstenite) provides the `WebSocketUpgrade` extractor. Extractors run before the upgrade, so putting the existing `Principal` extractor before `WebSocketUpgrade` answers 401 to an unauthenticated handshake with a normal HTTP response. `WebSocket` splits into a sink and a stream; `Message::Ping`/`Pong` are surfaced and axum answers pings automatically. `max_message_size` and `max_frame_size` bound what a client can send. Browsers send cookies on a WebSocket handshake and the handshake is not covered by CORS, so a session-cookie handshake must check the `Origin` header (cross-site WebSocket hijacking); bearer-token handshakes carry no ambient credentials and need no origin check.
- Source: https://docs.rs/axum/0.8.9/axum/extract/ws/index.html ; https://docs.rs/axum/0.8.9/axum/extract/ws/struct.WebSocketUpgrade.html ; https://www.rfc-editor.org/rfc/rfc6455#section-10.2 ; https://cheatsheetseries.owasp.org/cheatsheets/WebSocket_Security_Cheat_Sheet.html
- Decision: `axum = { version = "0.8", features = ["ws"] }` in `[workspace.dependencies]`; `GET /api/v1/live` authenticates with `Principal`, requires `Origin` equal to `APP_URL` for session cookies, caps messages at 4 KiB, pings every 20 s and drops peers that miss a pong, re-validates the credential and each subscription every 30 s, and closes with 1013 when a socket lags behind the hub (the page reconnects and refetches). `futures-util` (`std`, `sink`) provides `StreamExt`/`SinkExt` to split the socket. Rejected: Server-Sent Events and polling (owner's decision in the plan), a separate WebSocket service (a second deliverable for one route).

## R4 — WebSocket client in tests: `tokio-tungstenite` 0.29

- Question: how do the API tests open real WebSocket connections?
- Finding: the router tests use `tower::ServiceExt::oneshot`, which cannot upgrade a connection; the tests serve the router on an ephemeral port and connect with `tokio-tungstenite`, the same library and version axum already uses, so no second copy is compiled. It is a dev-dependency only.
- Source: https://docs.rs/tokio-tungstenite/0.29.0/tokio_tungstenite/ ; https://docs.rs/axum/0.8.9/axum/extract/ws/index.html
- Decision: `tokio-tungstenite = "0.29"` under `[dev-dependencies]` of `sdd-vault-api`.
