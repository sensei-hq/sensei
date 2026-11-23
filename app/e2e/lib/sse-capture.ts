// SSE capture for e2e: subscribe to a daemon stream from Node, collect
// frames, wait for a predicate. Plain fetch + reader — no DOM needed.

export interface CapturedSse {
  /** Parsed JSON payload, or the raw text when it is not JSON. */
  data: unknown;
  raw: string;
}

export interface SseCapture {
  events: CapturedSse[];
  errors: Error[];
  /** Reject unless `predicate` matches a frame within `timeoutMs`. */
  waitFor(predicate: (e: CapturedSse) => boolean, timeoutMs: number): Promise<CapturedSse>;
  close(): void;
}

export function captureSse(url: string, init?: { headers?: Record<string, string> }): SseCapture {
  const events: CapturedSse[] = [];
  const errors: Error[] = [];
  const waiters: Array<{
    predicate: (e: CapturedSse) => boolean;
    resolve: (e: CapturedSse) => void;
    reject: (e: Error) => void;
    timer: ReturnType<typeof setTimeout>;
  }> = [];
  const abort = new AbortController();
  let closed = false;

  const push = (raw: string) => {
    let data: unknown = raw;
    try {
      data = JSON.parse(raw);
    } catch {
      // Not JSON — keep the raw text (heartbeats, plain frames).
    }
    const event: CapturedSse = { data, raw };
    events.push(event);
    for (let i = waiters.length - 1; i >= 0; i--) {
      if (waiters[i].predicate(event)) {
        const [w] = waiters.splice(i, 1);
        clearTimeout(w.timer);
        w.resolve(event);
      }
    }
  };

  void (async () => {
    try {
      const res = await fetch(url, { signal: abort.signal, headers: init?.headers });
      const reader = res.body?.getReader();
      if (!reader) throw new Error('SSE response has no body');
      const decoder = new TextDecoder();
      let buffer = '';
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        let cut: number;
        while ((cut = buffer.indexOf('\n\n')) >= 0) {
          const frame = buffer.slice(0, cut);
          buffer = buffer.slice(cut + 2);
          const lines = frame
            .split('\n')
            .map((l) => (l.endsWith('\r') ? l.slice(0, -1) : l));
          const payload = lines
            .filter((l) => l.startsWith('data:'))
            .map((l) => l.slice(5).trimStart())
            .join('\n');
          if (payload) push(payload);
        }
      }
    } catch (e) {
      if (!closed) errors.push(e instanceof Error ? e : new Error(String(e)));
    }
  })();

  return {
    events,
    errors,
    waitFor: (predicate, timeoutMs) =>
      new Promise<CapturedSse>((resolve, reject) => {
        const found = events.find(predicate);
        if (found) {
          resolve(found);
          return;
        }
        const timer = setTimeout(() => {
          const i = waiters.findIndex((w) => w.resolve === resolve);
          if (i >= 0) waiters.splice(i, 1);
          reject(new Error(`sse-capture: timeout after ${timeoutMs}ms`));
        }, timeoutMs);
        waiters.push({ predicate, resolve, reject, timer });
      }),
    close: () => {
      closed = true;
      abort.abort();
    },
  };
}
