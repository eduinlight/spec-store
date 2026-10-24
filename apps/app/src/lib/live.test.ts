import { afterEach, beforeEach } from "vitest";
import {
  createLiveManager,
  type LiveEvent,
  type LiveSocket,
  MAX_DELAY_MS,
  reconnectDelay,
} from "./live";

class FakeSocket implements LiveSocket {
  static instances: FakeSocket[] = [];
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readyState = 0;
  sent: unknown[] = [];
  closedWith: number | undefined;

  constructor() {
    FakeSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(JSON.parse(data));
  }

  close(code?: number) {
    this.closedWith = code;
    this.readyState = 3;
  }

  open() {
    this.readyState = 1;
    this.onopen?.(new Event("open"));
  }

  receive(message: object) {
    this.onmessage?.(new MessageEvent("message", { data: JSON.stringify(message) }));
  }

  drop(code = 1006) {
    this.readyState = 3;
    this.onclose?.({ code } as CloseEvent);
  }
}

const latest = () => FakeSocket.instances[FakeSocket.instances.length - 1] as FakeSocket;

function setup() {
  return createLiveManager({ open: () => new FakeSocket(), random: () => 0 });
}

beforeEach(() => {
  FakeSocket.instances = [];
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

test("the first subscription opens one socket shared by every project and sends the subscribes on open", () => {
  const manager = setup();
  manager.subscribe("acme/demo", () => {});
  manager.subscribe("acme/other", () => {});
  expect(FakeSocket.instances).toHaveLength(1);
  latest().open();
  expect(latest().sent).toEqual([{ subscribe: "acme/demo" }, { subscribe: "acme/other" }]);
});

test("a subscription made while the socket is open is sent at once and leaving sends an unsubscribe", () => {
  const manager = setup();
  const first = manager.subscribe("acme/demo", () => {});
  latest().open();
  const second = manager.subscribe("acme/other", () => {});
  expect(latest().sent).toContainEqual({ subscribe: "acme/other" });
  second();
  expect(latest().sent).toContainEqual({ unsubscribe: "acme/other" });
  first();
  expect(latest().closedWith).toBe(1000);
});

test("the server confirmation marks a project as live and drives the state listeners", () => {
  const manager = setup();
  const states: boolean[] = [];
  manager.onState(() => states.push(manager.isSubscribed("acme/demo")));
  manager.subscribe("acme/demo", () => {});
  latest().open();
  expect(manager.isSubscribed("acme/demo")).toBe(false);
  latest().receive({ subscribed: "acme/demo" });
  expect(manager.isSubscribed("acme/demo")).toBe(true);
  latest().drop();
  expect(manager.isSubscribed("acme/demo")).toBe(false);
  expect(states).toEqual([true, false]);
});

test("events reach only the listeners of their project", () => {
  const manager = setup();
  const demo: LiveEvent[] = [];
  const other: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => demo.push(event));
  manager.subscribe("acme/other", (event) => other.push(event));
  latest().open();
  latest().receive({ project: "acme/demo", audit_id: 9, action: "task.done", result: "ok" });
  expect(demo).toEqual([
    { type: "write", project: "acme/demo", auditId: 9, action: "task.done", result: "ok" },
  ]);
  expect(other).toEqual([]);
});

test("a refused subscription is not live", () => {
  const manager = setup();
  manager.subscribe("acme/demo", () => {});
  latest().open();
  latest().receive({ subscribed: "acme/demo" });
  latest().receive({ error: "not_found", project: "acme/demo" });
  expect(manager.isSubscribed("acme/demo")).toBe(false);
});

test("a resync message asks every listener to refetch", () => {
  const manager = setup();
  const seen: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => seen.push(event));
  latest().open();
  latest().receive({ resync: true });
  expect(seen).toEqual([{ type: "resync" }]);
});

test("the backoff doubles from one second up to thirty with jitter", () => {
  expect(reconnectDelay(0, 0)).toBe(1000);
  expect(reconnectDelay(1, 0)).toBe(2000);
  expect(reconnectDelay(4, 0)).toBe(16000);
  expect(reconnectDelay(5, 0)).toBe(MAX_DELAY_MS);
  expect(reconnectDelay(9, 0)).toBe(MAX_DELAY_MS);
  expect(reconnectDelay(0, 1)).toBe(1250);
});

test("a dropped socket reconnects with growing delays, resubscribes and tells listeners to resync", () => {
  const manager = setup();
  const seen: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => seen.push(event));
  latest().open();
  latest().drop();
  expect(FakeSocket.instances).toHaveLength(1);
  vi.advanceTimersByTime(999);
  expect(FakeSocket.instances).toHaveLength(1);
  vi.advanceTimersByTime(1);
  expect(FakeSocket.instances).toHaveLength(2);
  latest().drop();
  vi.advanceTimersByTime(1999);
  expect(FakeSocket.instances).toHaveLength(2);
  vi.advanceTimersByTime(1);
  expect(FakeSocket.instances).toHaveLength(3);
  expect(seen).toEqual([]);
  latest().open();
  expect(latest().sent).toEqual([{ subscribe: "acme/demo" }]);
  expect(seen).toEqual([{ type: "resync" }]);
  latest().drop();
  vi.advanceTimersByTime(1000);
  expect(FakeSocket.instances).toHaveLength(4);
});

test("a socket that fell behind (1013) reconnects and the listeners refetch", () => {
  const manager = setup();
  const seen: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => seen.push(event));
  latest().open();
  latest().drop(1013);
  vi.advanceTimersByTime(1000);
  latest().open();
  expect(FakeSocket.instances).toHaveLength(2);
  expect(seen).toEqual([{ type: "resync" }]);
});

test("an ended credential (1008) stops reconnecting and asks pages to refetch so the session guard acts", () => {
  const manager = setup();
  const seen: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => seen.push(event));
  latest().open();
  latest().drop(1008);
  vi.advanceTimersByTime(60000);
  expect(FakeSocket.instances).toHaveLength(1);
  expect(seen).toEqual([{ type: "resync" }]);
});

test("leaving every project cancels a pending reconnect", () => {
  const manager = setup();
  const leave = manager.subscribe("acme/demo", () => {});
  latest().open();
  latest().drop();
  leave();
  vi.advanceTimersByTime(60000);
  expect(FakeSocket.instances).toHaveLength(1);
});

test("ignores messages that are not JSON objects", () => {
  const manager = setup();
  const seen: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => seen.push(event));
  latest().open();
  latest().onmessage?.(new MessageEvent("message", { data: "not json" }));
  latest().onmessage?.(new MessageEvent("message", { data: "7" }));
  expect(seen).toEqual([]);
});

test("the confirmation of a subscription asks its listeners to refetch what happened before it", () => {
  const manager = setup();
  const demo: LiveEvent[] = [];
  const other: LiveEvent[] = [];
  manager.subscribe("acme/demo", (event) => demo.push(event));
  manager.subscribe("acme/other", (event) => other.push(event));
  latest().open();
  latest().receive({ subscribed: "acme/demo" });
  expect(demo).toEqual([{ type: "resync" }]);
  expect(other).toEqual([]);
});
