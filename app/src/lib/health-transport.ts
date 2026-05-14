// app/src/lib/health-transport.ts

import type { HealthPayload, HealthEvent } from './health-types.js';

// ── Interface ─────────────────────────────────────────────────────────────────

export interface HealthTransport {
  /** Sync fast path. Returns the current health state. */
  check(): Promise<HealthPayload>;

  /** Streaming resolve. `onEvent` fires for every HealthEvent.
   *  Resolves with the terminal HealthPayload (which is also delivered
   *  as a final `kind: 'report'` event before this promise resolves). */
  resolve(current: HealthPayload, onEvent: (ev: HealthEvent) => void): Promise<HealthPayload>;
}

// ── RealTransport ─────────────────────────────────────────────────────────────

/**
 * Production transport.  Calls the Tauri sidecar via invoke / listen.
 *
 * Tauri APIs are lazy-imported inside each method so that this module can be
 * loaded in headless/SSR test contexts without the Tauri IPC bridge present.
 */
export class RealTransport implements HealthTransport {
  async check(): Promise<HealthPayload> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<HealthPayload>('health_check');
  }

  async resolve(
    _current: HealthPayload,
    onEvent: (ev: HealthEvent) => void,
  ): Promise<HealthPayload> {
    const { invoke } = await import('@tauri-apps/api/core');
    const { listen } = await import('@tauri-apps/api/event');

    return new Promise<HealthPayload>((resolveFn, rejectFn) => {
      let unlisten: (() => void) | null = null;
      let settled = false;

      const cleanup = () => {
        try { unlisten?.(); } catch { /* ignore */ }
      };

      listen<HealthEvent>('health', (e) => {
        if (settled) return;
        const ev = e.payload;
        onEvent(ev);
        if (ev.kind === 'report') {
          settled = true;
          cleanup();
          resolveFn(ev.payload);
        }
      }).then((un) => {
        unlisten = un;
        // Listener is wired — now kick off the streaming resolve on the Rust side.
        invoke<void>('health_check_and_resolve').catch((err) => {
          if (settled) return;
          settled = true;
          cleanup();
          rejectFn(err);
        });
      }).catch((err) => {
        if (settled) return;
        settled = true;
        rejectFn(err);
      });
    });
  }
}

// ── MockTransport ─────────────────────────────────────────────────────────────

export interface MockTransportOpts {
  checkPayload: HealthPayload;
  resolveEvents?: HealthEvent[];    // default: []
  resolveTerminal?: HealthPayload;  // default: checkPayload
}

export class MockTransport implements HealthTransport {
  readonly checkCalls: Array<[]> = [];
  readonly resolveCalls: Array<{ current: HealthPayload }> = [];

  constructor(private readonly opts: MockTransportOpts) {}

  async check(): Promise<HealthPayload> {
    this.checkCalls.push([]);
    return this.opts.checkPayload;
  }

  async resolve(
    current: HealthPayload,
    onEvent: (ev: HealthEvent) => void,
  ): Promise<HealthPayload> {
    this.resolveCalls.push({ current });
    for (const ev of this.opts.resolveEvents ?? []) onEvent(ev);
    return this.opts.resolveTerminal ?? this.opts.checkPayload;
  }
}
