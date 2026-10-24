export class FakeWebSocket {
  static instances: FakeWebSocket[] = [];
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readyState = 0;
  sent: Record<string, unknown>[] = [];
  url: string;

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(JSON.parse(data));
  }

  close() {
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

export const lastSocket = (): FakeWebSocket | undefined =>
  FakeWebSocket.instances[FakeWebSocket.instances.length - 1];

export function connectLive(address: string): FakeWebSocket {
  const socket = lastSocket();
  if (!socket) throw new Error("no socket was opened");
  socket.open();
  socket.receive({ subscribed: address });
  return socket;
}
