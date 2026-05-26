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
      // `health::check_and_resolve` emits TWO Report events when the
      // system needs action: an INITIAL one right after check() (still
      // carrying default_remedy), and a TERMINAL one after resolve()
      // completes (with per-component remedies). We must only settle on
      // the terminal one — otherwise we unlisten before any Remedy or
      // Component event from the resolve phase reaches us, and the UI
      // ends up stuck with the initial state's default_remedy.
      //
      // Heuristic: settle when either
      //   (a) we see a Report whose status is 'ok' — the system was
      //       already healthy, no resolve will run, so this IS terminal;
      //   (b) we've seen Phase(Resolving) — the next Report is the
      //       terminal one.
      let resolvingStarted = false;

      const cleanup = () => {
        try { unlisten?.(); } catch { /* ignore */ }
      };

      listen<HealthEvent>('health', (e) => {
        if (settled) return;
        const ev = e.payload;
        onEvent(ev);
        if (ev.kind === 'phase' && ev.phase === 'resolving') {
          resolvingStarted = true;
          return;
        }
        if (ev.kind === 'report') {
          const isTerminal = ev.payload.status === 'ok' || resolvingStarted;
          if (!isTerminal) return;  // initial report — keep listening
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
    const terminal = this.opts.resolveTerminal ?? this.opts.checkPayload;
    // Mirror RealTransport: the Rust pipeline always emits a terminal
    // `report` event before returning, which the consumer's onEvent
    // handler turns into `apply(payload)`. Without this, callers that
    // drive state purely from streaming events (the post-2026-05-19
    // streaming `init()` path) would never see the final payload.
    onEvent({ kind: 'report', payload: terminal });
    return terminal;
  }
}
