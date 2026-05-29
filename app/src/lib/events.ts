/**
 * Generic SSE event manager — subscriber/observer pattern.
 *
 * Connects to an SSE endpoint, parses events, and dispatches to handlers.
 * Auto-reconnects on error. Disconnects when last handler unsubscribes.
 */

export type EventHandler<T> = (event: T) => void;

export class EventManager<T> {
  private source: EventSource | null = null;
  private handlers: EventHandler<T>[] = [];
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private url: string,
    private parse: (data: string) => T,
    private reconnectMs = 3000,
  ) {}

  /** Subscribe a handler. Returns an unsubscribe function. */
  subscribe(handler: EventHandler<T>): () => void {
    this.handlers.push(handler);
    if (!this.source) this.connect();
    return () => this.unsubscribe(handler);
  }

  /**
   * Resolve once the EventSource is OPEN (or immediately if already open).
   * Use after subscribe() and before triggering server-side work whose
   * broadcast events you need to capture — without this, the POST that
   * kicks the work can land before the SSE handshake completes, and the
   * first wave of events fans out to zero subscribers.
   */
  ready(timeoutMs = 5000): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.source?.readyState === EventSource.OPEN) { resolve(); return; }
      if (!this.source) { reject(new Error('not connected')); return; }
      const src = this.source;
      const t = setTimeout(() => reject(new Error('sse open timeout')), timeoutMs);
      src.addEventListener('open', () => { clearTimeout(t); resolve(); }, { once: true });
    });
  }

  /** Number of active subscribers. */
  get subscriberCount(): number {
    return this.handlers.length;
  }

  /** Whether the EventSource is connected. */
  get connected(): boolean {
    return this.source?.readyState === EventSource.OPEN;
  }

  /** Tear down — remove all handlers and close connection. */
  destroy() {
    this.handlers = [];
    this.disconnect();
  }

  private unsubscribe(handler: EventHandler<T>) {
    this.handlers = this.handlers.filter(h => h !== handler);
    if (this.handlers.length === 0) this.disconnect();
  }

  private connect() {
    this.clearReconnect();
    this.source = new EventSource(this.url);

    this.source.onmessage = (e) => {
      try {
        const parsed = this.parse(e.data);
        for (const handler of this.handlers) {
          handler(parsed);
        }
      } catch {
        // Parse error — skip event, don't crash
      }
    };

    this.source.onerror = () => {
      this.disconnect();
      if (this.handlers.length > 0) {
        this.reconnectTimer = setTimeout(() => this.connect(), this.reconnectMs);
      }
    };
  }

  private disconnect() {
    this.clearReconnect();
    this.source?.close();
    this.source = null;
  }

  private clearReconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }
}
