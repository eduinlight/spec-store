import "@testing-library/jest-dom/vitest";
import { configure } from "@testing-library/react";
import { FakeWebSocket } from "./fake-socket";

configure({ asyncUtilTimeout: 5000 });

Object.defineProperty(globalThis, "WebSocket", { value: FakeWebSocket, configurable: true });
