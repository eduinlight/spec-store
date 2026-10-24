import { useEffect, useRef, useSyncExternalStore } from "react";
import { useRevalidator } from "react-router";

export type LiveEvent =
  | { type: "resync" }
  | { type: "write"; project: string; auditId: number; action: string; result: string };

export type LiveListener = (event: LiveEvent) => void;

export interface LiveSocket {
  onopen: ((event: Event) => void) | null;
  onmessage: ((event: MessageEvent) => void) | null;
  onclose: ((event: CloseEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  readonly readyState: number;
  send: (data: string) => void;
  close: (code?: number) => void;
}

export interface LiveDependencies {
  open: () => LiveSocket;
  random: () => number;
}

export interface LiveManager {
  subscribe: (address: string, listener: LiveListener) => () => void;
  isSubscribed: (address: string) => boolean;
  onState: (listener: () => void) => () => void;
}

export const MIN_DELAY_MS = 1000;
export const MAX_DELAY_MS = 30000;
export const JITTER_RATIO = 0.25;
export const REVALIDATE_DEBOUNCE_MS = 300;
export const CLOSE_POLICY = 1008;

const SOCKET_OPEN = 1;

export const reconnectDelay = (attempt: number, random: number): number => {
  const base = Math.min(MAX_DELAY_MS, MIN_DELAY_MS * 2 ** attempt);
  return base + Math.round(base * JITTER_RATIO * random);
};

interface ServerMessage {
  subscribed?: string;
  unsubscribed?: string;
  error?: string;
  resync?: boolean;
  project?: string;
  audit_id?: number;
  action?: string;
  result?: string;
}

const parse = (data: unknown): ServerMessage | null => {
  if (typeof data !== "string") return null;
  try {
    const value: unknown = JSON.parse(data);
    return typeof value === "object" && value !== null ? (value as ServerMessage) : null;
  } catch {
    return null;
  }
};

export function createLiveManager(dependencies: LiveDependencies): LiveManager {
  const listeners = new Map<string, Set<LiveListener>>();
  const stateListeners = new Set<() => void>();
  const confirmed = new Set<string>();
  let socket: LiveSocket | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let attempt = 0;
  let interrupted = false;
  let stopped = false;

  const notifyState = () => {
    for (const listener of stateListeners) listener();
  };

  const emit = (address: string, event: LiveEvent) => {
    for (const listener of listeners.get(address) ?? []) listener(event);
  };

  const emitAll = (event: LiveEvent) => {
    for (const address of listeners.keys()) emit(address, event);
  };

  const send = (message: object) => {
    if (socket && socket.readyState === SOCKET_OPEN) socket.send(JSON.stringify(message));
  };

  const connect = () => {
    timer = null;
    const current = dependencies.open();
    socket = current;
    current.onopen = () => {
      if (socket !== current) return;
      attempt = 0;
      for (const address of listeners.keys()) send({ subscribe: address });
      if (interrupted) {
        interrupted = false;
        emitAll({ type: "resync" });
      }
    };
    current.onmessage = (event) => {
      if (socket !== current) return;
      const message = parse(event.data);
      if (!message) return;
      if (typeof message.subscribed === "string") {
        confirmed.add(message.subscribed);
        notifyState();
      } else if (typeof message.unsubscribed === "string") {
        confirmed.delete(message.unsubscribed);
        notifyState();
      } else if (message.resync === true) {
        emitAll({ type: "resync" });
      } else if (typeof message.error === "string") {
        if (typeof message.project === "string") {
          confirmed.delete(message.project);
          notifyState();
        }
      } else if (typeof message.project === "string") {
        emit(message.project, {
          type: "write",
          project: message.project,
          auditId: message.audit_id ?? 0,
          action: message.action ?? "",
          result: message.result ?? "",
        });
      }
    };
    current.onclose = (event) => {
      if (socket !== current) return;
      socket = null;
      interrupted = true;
      confirmed.clear();
      notifyState();
      if (event.code === CLOSE_POLICY) {
        stopped = true;
        emitAll({ type: "resync" });
        return;
      }
      if (listeners.size === 0) return;
      timer = setTimeout(connect, reconnectDelay(attempt, dependencies.random()));
      attempt += 1;
    };
  };

  const disconnect = () => {
    if (timer) clearTimeout(timer);
    timer = null;
    attempt = 0;
    interrupted = false;
    stopped = false;
    const current = socket;
    socket = null;
    confirmed.clear();
    current?.close(1000);
  };

  const subscribe = (address: string, listener: LiveListener) => {
    const existing = listeners.get(address);
    if (existing) {
      existing.add(listener);
    } else {
      listeners.set(address, new Set([listener]));
      send({ subscribe: address });
    }
    if (!socket && !timer && !stopped) connect();
    return () => {
      const group = listeners.get(address);
      if (!group) return;
      group.delete(listener);
      if (group.size > 0) return;
      listeners.delete(address);
      confirmed.delete(address);
      send({ unsubscribe: address });
      notifyState();
      if (listeners.size === 0) disconnect();
    };
  };

  return {
    subscribe,
    isSubscribed: (address) => confirmed.has(address),
    onState: (listener) => {
      stateListeners.add(listener);
      return () => {
        stateListeners.delete(listener);
      };
    },
  };
}

const socketUrl = () => `${window.location.origin.replace(/^http/, "ws")}/api/v1/live`;

export const live = createLiveManager({
  open: () => new WebSocket(socketUrl()),
  random: Math.random,
});

export function useLiveProject(address: string, onEvent: LiveListener): boolean {
  const handler = useRef(onEvent);
  useEffect(() => {
    handler.current = onEvent;
  });
  useEffect(() => live.subscribe(address, (event) => handler.current(event)), [address]);
  return useSyncExternalStore(
    live.onState,
    () => live.isSubscribed(address),
    () => false,
  );
}

export function useLiveRevalidate(address: string): boolean {
  const revalidator = useRevalidator();
  const revalidate = useRef(revalidator.revalidate);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    revalidate.current = revalidator.revalidate;
  });
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );
  return useLiveProject(address, () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      timer.current = null;
      void revalidate.current();
    }, REVALIDATE_DEBOUNCE_MS);
  });
}
