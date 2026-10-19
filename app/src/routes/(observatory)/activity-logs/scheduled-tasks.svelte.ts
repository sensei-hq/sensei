// Observatory · Logs — background-task registry (#96).
//
// Pure derivations for the "Background tasks" panel: what each daemon worker
// is, when it last ran, how it went (last_ok / last_error), when it runs next
// and how long a pass takes (avg_ms), read from `GET /api/tasks/scheduled`.

import type { ScheduledTask } from '$lib/types.js';
import { relativeTime } from './activity-logs.svelte.js';
import type { ApiResult } from '$lib/api.js';

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

/** Health summary for a worker: the em dash until its first recorded pass. */
export function taskHealth(task: ScheduledTask): string {
  if (task.last_ok == null) return EM_DASH;
  return task.last_ok ? 'ok' : 'failing';
}

/** The worker's last error, or the em dash when none is recorded. */
export function taskError(task: ScheduledTask): string {
  return task.last_error ?? EM_DASH;
}

/** When the worker runs next, read forward: "in 2m", or "due now" once that
 *  moment has passed (it runs on the next poll). The em dash when there is no
 *  next run — the daemon sends null for a disabled schedule. `relativeTime`
 *  only reads backward, which clamped a future time to "just now" (#96). */
export function taskNextRun(nowMs: number, task: ScheduledTask): string {
  if (task.next_run_at == null) return EM_DASH;
  const t = Date.parse(task.next_run_at);
  if (Number.isNaN(t)) return task.next_run_at;
  const secs = Math.round((t - nowMs) / 1000);
  if (secs <= 5) return 'due now';
  if (secs < 60) return `in ${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `in ${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `in ${hrs}h`;
  return `in ${Math.floor(hrs / 24)}d`;
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

/** ISO weekday names, indexed 1..7 so the wire's numbers index directly. */
const DAY_NAMES = ['', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

/** A day mask as "Mon-Fri" / "Sat, Sun" / "Wed".
 *
 *  Contiguous runs collapse to a range because that is how people say it — a
 *  weekday schedule reading "Mon, Tue, Wed, Thu, Fri" is correct and unreadable.
 */
function dayLabel(days: number[]): string {
  const sorted = [...new Set(days)].filter((d) => d >= 1 && d <= 7).sort((a, b) => a - b);
  if (sorted.length === 0) return '';
  const runs: number[][] = [];
  for (const d of sorted) {
    const last = runs[runs.length - 1];
    if (last && d === last[last.length - 1] + 1) last.push(d);
    else runs.push([d]);
  }
  return runs
    .map((r) => (r.length >= 3 ? `${DAY_NAMES[r[0]]}-${DAY_NAMES[r[r.length - 1]]}` : r.map((d) => DAY_NAMES[d]).join(', ')))
    .join(', ');
}

/** WHEN a worker is allowed to run — "off" / "any time" / "Mon-Fri 09:00-17:00".
 *
 *  Three states that must stay distinguishable, because the wrong one is a
 *  support call:
 *  - `off` — disabled. Says so plainly rather than showing a cadence that will
 *    never fire.
 *  - `any time` — no window and no day mask. An unset mask means EVERY day, so
 *    rendering it as an empty cell would read as "nothing scheduled".
 *  - a window and/or days, as configured.
 *
 *  The window string is the daemon's own label and is never re-derived here;
 *  one that wraps midnight (`22:00-05:00`) is meaningful and passes through
 *  untouched. */
export function taskSchedule(task: ScheduledTask): string {
  if (!task.enabled) return 'off';
  const days = task.days?.length ? dayLabel(task.days) : '';
  const parts = [days, task.window ?? ''].filter(Boolean);
  return parts.length ? parts.join(' ') : 'any time';
}

/** Average run duration as "12ms" / "1.4s", or the em dash when unrecorded. */
export function taskAvg(task: ScheduledTask): string {
  const ms = task.avg_ms;
  if (ms == null || ms < 0) return EM_DASH;
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/** A worker's log tail, opened from its row (#96). The loader is injected so
 *  the state is testable; a failed read is an error, never an empty log. */
export class TaskLog {
  open = $state(false);
  loading = $state(false);
  lines = $state<string[]>([]);
  error = $state<string | null>(null);

  constructor(private readonly load: () => Promise<ApiResult<{ lines: string[] }>>) {}

  async toggle(): Promise<void> {
    this.open = !this.open;
    if (!this.open) return;
    this.loading = true;
    this.error = null;
    const res = await this.load();
    this.loading = false;
    if (res.ok) {
      this.lines = res.data.lines;
    } else {
      this.lines = [];
      this.error = `Could not read the log: ${res.error.message || `HTTP ${res.error.status}`}`;
    }
  }
}
