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
