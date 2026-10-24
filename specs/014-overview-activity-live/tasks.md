# Tasks — Spec 014

Plan: `specs/014-overview-activity-live/plan.md` · Date: 2026-10-24

- [x] T01 (FR-1 to FR-5) Core read models: overview, activity, active specs with next task and agent prompt — Done when: core tests green
- [x] T02 (FR-6, NFR-2) `EventPublisher` port, publish after commit for writes and refusals; Redis in compose and `.env.dist` — Done when: core tests green
- [ ] T03 (FR-6, FR-7, NFR-1) API: Redis publisher, LiveHub, `/api/v1/live` WebSocket with authorization; MCP publishes; view endpoints; `make openapi` — Done when: API and MCP tests green
- [ ] T04 (FR-1 to FR-3, FR-8) Overview screen with period, KPIs, chart and attention list — Done when: app tests green
- [ ] T05 (FR-4, FR-8) Active specs screen with Copy agent prompt — Done when: app tests green
- [ ] T06 (FR-5, FR-6, FR-7, FR-8) Activity screen with filters and live updates; live hook used by Overview — Done when: app tests green
- [ ] T07 CD (Redis for staging and production, REDIS_URL), gate, Playwright live check, `validation.md` — Done when: gate green
