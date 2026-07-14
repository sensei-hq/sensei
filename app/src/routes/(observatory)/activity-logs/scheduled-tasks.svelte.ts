// Observatory · Logs — background-task registry (#96).
//
// Pure derivations for the "Background tasks" panel: what each daemon worker
// is and when it last ran, read from `GET /api/tasks/scheduled`. The run-health
// fields (last_ok / last_error / next_run_at / interval_secs / avg_ms) are all
// null today — the daemon doesn't yet record run outcomes — so they render as a
// muted em dash, never a fabricated "healthy" or a made-up number.
//
// Everything here is pure and unit-tested; `relativeTime` is reused from the
// log stream so "3h ago" reads identically in both places.

import type { ScheduledTask } from '$lib/types.js';
import { relativeTime } from './activity-logs.svelte.js';

/** Muted placeholder for a field the daemon hasn't recorded yet. */
export const EM_DASH = '—';

/** When a worker has never run, its last-run cell reads "never" rather than a
 *  relative time. */
export const NEVER = 'never';

/** Relative last-run time for a worker, or "never" when it persists no
 *  watermark. Reuses the log stream's `relativeTime`, so an unparseable
 *  timestamp falls back to the raw string there. */
export function taskLastRun(nowMs: number, task: ScheduledTask): string {
  if (task.last_run_at == null) return NEVER;
  return relativeTime(nowMs, task.last_run_at);
}

/** Health summary for a worker. Every field is null until the daemon records
 *  run outcomes, so this is always the em dash today — kept a helper so the
 *  moment the wire starts sending outcomes, the formatting lives in one place
 *  and the component stays a pure template. */
export function taskHealth(task: ScheduledTask): string {
  if (task.last_ok == null) return EM_DASH;
  return task.last_ok ? 'ok' : 'failing';
}

/** The worker's last error, or the em dash when none is recorded. */
export function taskError(task: ScheduledTask): string {
  return task.last_error ?? EM_DASH;
}

/** Relative next-run time, or the em dash when the daemon doesn't schedule a
 *  concrete next tick yet. */
export function taskNextRun(nowMs: number, task: ScheduledTask): string {
  if (task.next_run_at == null) return EM_DASH;
  return relativeTime(nowMs, task.next_run_at);
}

/** The worker's cadence as "every 5m" / "every 2h", or the em dash when no
 *  interval is recorded. */
export function taskInterval(task: ScheduledTask): string {
  const secs = task.interval_secs;
  if (secs == null || secs <= 0) return EM_DASH;
  if (secs < 60) return `every ${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `every ${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `every ${hrs}h`;
  const days = Math.floor(hrs / 24);
  return `every ${days}d`;
}

/** Average run duration as "12ms" / "1.4s", or the em dash when unrecorded. */
export function taskAvg(task: ScheduledTask): string {
  const ms = task.avg_ms;
  if (ms == null || ms < 0) return EM_DASH;
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}
