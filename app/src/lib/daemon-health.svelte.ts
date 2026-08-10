// app/src/lib/daemon-health.svelte.ts
//
// Polls the daemon's HTTP /health for its own DB-connection mode and exposes it
// for a global status banner. This is distinct from health-state.svelte.ts: that
// one drives the setup/bootstrap screen off the in-app sidecar check (blind to
// the daemon's pool). Only the daemon's /health reports `daemonDbMode`, so a
// short "degraded → recovering" window (daemon lost the cold-boot race and is
// self-healing) is visible to the running app without a restart.

import type { DaemonDbMode } from './health-types.js';

export class DaemonHealth {
  /** Latest daemon-reported mode. `undefined` = unknown (daemon down, or a
   *  payload that didn't carry the field) → no banner. */
  dbMode = $state<DaemonDbMode | undefined>(undefined);

  #timer: ReturnType<typeof setInterval> | null = null;

  get isDegraded(): boolean {
    return this.dbMode === 'degraded';
  }

  /** Update from a daemon /health payload. Pure — the poll and tests share it. */
  apply(payload: { daemonDbMode?: DaemonDbMode }): void {
    this.dbMode = payload.daemonDbMode;
  }

  /** One poll of the daemon's /health. Lazy-imports the API + app state so this
   *  module stays cheap to import (e.g. from unit tests) and only loads the
   *  daemon client when a poll actually runs. */
  async poll(): Promise<void> {
    const [{ senseiApi }, { appState }] = await Promise.all([
      import('./api.js'),
      import('./appstate.svelte.js'),
    ]);
    const payload = await senseiApi(appState.port).getHealth();
    this.apply(payload as { daemonDbMode?: DaemonDbMode });
    // Degraded is a transient cold-boot recovery window. Once the daemon reports
    // a working pool there's nothing left to watch, so stop — /health runs a full
    // component probe (spawns psql/pg_isready/brew), too heavy to poll forever.
    // A daemon that's down returns {} cheaply (connection refused), so we keep
    // polling harmlessly until it comes up and reports full.
    if (this.dbMode === 'full') this.stop();
  }

  /** Begin polling. Idempotent. Polls immediately, then every `intervalMs`. */
  start(intervalMs = 5000): void {
    if (this.#timer) return;
    void this.poll();
    this.#timer = setInterval(() => {
      void this.poll();
    }, intervalMs);
  }

  stop(): void {
    if (this.#timer) {
      clearInterval(this.#timer);
      this.#timer = null;
    }
  }
}

export const daemonHealth = new DaemonHealth();
