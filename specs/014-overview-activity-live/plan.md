# Technical plan — Spec 014

Spec: `specs/014-overview-activity-live/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

Two parts. **Read models**: overview metrics, tasks completed per day and client, attention items, the active
specs view with the next task and the agent prompt, and the activity feed, all derived from the audit trail
(spec 010 gives every entry its client, origin and result) and the markdown. **Live channel**: after every audit
insert commits (writes and refusals, on API and MCP instances), core publishes a `ProjectEvent { project_id,
audit_id, action, result }` through an `EventPublisher` port. The Redis adapter publishes to channel
`sddvault:project:{id}`. Each API instance runs one `LiveHub`: a single Redis pattern subscription
(`sddvault:project:*`) fanned out in-process to WebSocket sessions through `tokio::sync::broadcast`, filtered per
session by the projects it subscribed to after an authorization check. Browsers connect to `GET /api/v1/live`
(WebSocket upgrade with the session cookie), send `{"subscribe": "<org>/<project>"}`, and receive
`{"project", "audit_id", "action", "result"}`; on each event the page refetches its data. On reconnect the page
refetches everything.

## 2. Technical context

- New Rust dependencies: `redis` (async, `tokio-comp`, `aio`) — the Redis client; `axum` feature `ws`
  (tokio-tungstenite through axum) — WebSockets. Both justified in §7.
- Redis 8 in `docker-compose.yml` (dev, tests) and in the CD base (staging, production); `REDIS_URL` for API and
  MCP. Without `REDIS_URL` the publisher is a no-op and the hub delivers only events of its own instance
  (single-instance development still works).
- App: `useLiveProject(address, onEvent)` hook with exponential backoff (1 s → 30 s), used by Overview,
  Active specs and Activity.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 2 | yes | Redis admitted by v3.0 for live fan-out; WebSockets through axum's built-in support |
| 4 | yes | `EventPublisher` port in core (adapters: Redis, in-process, no-op); `LiveHub` is the only subscriber; justified because writes happen in two deliverables and browsers connect to N instances |
| 6 | yes | PostgreSQL stays the record; Redis carries only notifications that the page turns into refetches |
| 7 | yes | A session subscribes only to projects it can read (checked on subscribe; events carry no document content) |

## 4. Files

```
packages/core/src/events.rs                 ProjectEvent, EventPublisher trait, NoopPublisher, InProcessPublisher
packages/core/src/store/mod.rs              publish after the audit transaction commits (writes and refusals)
packages/core/src/store/views/overview.rs   metrics, completed-per-day by client, attention items
packages/core/src/store/views/activity.rs   feed rows with words, filters, conflict-then-saved grouping
packages/core/src/store/views/active.rs     active spec, next task, agent prompt, drafts with questions
apps/api/src/live.rs                        RedisPublisher, LiveHub (one pattern subscription per instance), WS route
apps/mcp/src/main.rs                        RedisPublisher when REDIS_URL is set
apps/api/src/routes/views.rs                GET .../overview?days=, .../active, .../activity?filter=&before=
apps/app/src/lib/live.ts                    useLiveProject with backoff
apps/app/src/routes/overview.tsx, active-specs.tsx, activity.tsx
docker-compose.yml, .env.dist               redis service, REDIS_URL
```

## 5. Decisions

- **Redis pub/sub for live fan-out**; rejected: PostgreSQL LISTEN/NOTIFY (one long-lived connection per instance
  outside the pool and an 8 KB payload limit; workable, but Redis is the platform's chosen live bus and SV-13
  presence needs it), RabbitMQ (durable delivery is not needed for "something changed, refetch").
- **WebSockets** (owner's decision); rejected: SSE and polling.
- **Notifications, not data, on the wire**: the page refetches through the authorized API; nothing sensitive
  travels on the bus.

## 6. Test strategy

Core: views on seeded audit trails; publisher called after commit, not on rollback. API: overview, active and
activity endpoints; WebSocket: unauthenticated upgrade refused, subscribe to a foreign project refused, a write
through the API reaches a subscribed socket within 2 s, a refusal too; two hubs on the same Redis both deliver.
App: hook reconnects with backoff; pages refetch on events. Playwright: open Activity, write through the API,
see the row appear without reloading.
