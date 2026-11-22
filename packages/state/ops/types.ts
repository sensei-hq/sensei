// The ops domain's wire shapes: queue listings, feeds, schedules, and sync
// state — verbatim daemon JSON (snake_case); states and selectors shape
// views from these.

export interface OpsTaskView {
  id: number;
  kind: string;
  path: string;
  folder_path: string;
  parent_task_id: number | null;
  retry_number: number;
  wait_ms: number | null;
  running_ms: number | null;
  blocked_on: number[];
  budget_secs: number | null;
  nearing_budget: boolean;
}

export interface OpsBackoffView {
  task_id: number;
  kind: string;
  path: string;
  folder_path: string;
  attempt: number;
  retry_in_ms: number;
}

export interface OpsKindDepth {
  pending: number;
  blocked: number;
  running: number;
}

export interface OpsQueueListing {
  pending: OpsTaskView[];
  blocked: OpsTaskView[];
  running: OpsTaskView[];
  backoff: OpsBackoffView[];
  totals: { pending: number; blocked: number; running: number };
  by_kind: Record<string, OpsKindDepth>;
  truncated: boolean;
}

export interface OpsFailedTask {
  taskId: number;
  parentTaskId: number | null;
  kind: string;
  folderPath: string;
  path: string;
  status: string;
  itemsProcessed: number;
  durationMs: number | null;
  retryNumber: number;
  error: string | null;
  startedAt: string;
  completedAt: string | null;
}

export interface OpsAbandonedFeed {
  days: number;
  by_day: Array<{ day: string; task_kind: string; count: number }>;
  worst: Array<{ task_kind: string; path: string; count: number; last_at: string }>;
  tasks: OpsFailedTask[];
}

export interface OpsIndexError {
  folder_id: string;
  file_path: string;
  error: string;
  adapter: string | null;
  phase: string | null;
  created_at: string;
}

export interface OpsScheduledTask {
  name: string;
  description: string;
  enabled: boolean;
  interval_secs: number;
  window: string | null;
  days: number[];
  last_run_at: string | null;
  last_ok: boolean | null;
  last_error: string | null;
  avg_ms: number | null;
  running_since: string | null;
  last_skip_reason: string | null;
  last_skip_at: string | null;
  overrun_limit_secs: number;
  next_run_at: string | null;
}

export interface OpsSchedulePatch {
  enabled?: boolean;
  interval_secs?: number;
  window_start?: string | null;
  window_end?: string | null;
  days?: number[] | null;
}

export interface OpsScheduleLog {
  worker: string;
  file: string;
  lines: string[];
}

export interface OpsSyncStateRow {
  entity: string;
  entity_key: string;
  direction: string;
  state: string;
  last_error: string | null;
  attempted_at: string | null;
  synced_at: string | null;
  updated_at: string;
  persona: string | null;
  remedy: string | null;
}

/** `GET /api/dojo/sync-state`: rows worst-first in `entities`, with a by-state tally. */
export interface OpsSyncStateFeed {
  count: number;
  counts: Record<string, number>;
  entities: OpsSyncStateRow[];
}

/** One wave row: transcript/metric/sync waves keyed by dispatcher (or pass). */
export interface OpsWave {
  wave: string;
  key: string;
  total: number;
  settled: number;
  done: boolean;
}

/** `GET /api/tasks/{id}` for a task with execution rows: newest attempt first, children, and their rollup. */
export interface OpsTaskDetail {
  taskId: number;
  current: OpsFailedTask | null;
  attempts: OpsFailedTask[];
  children: OpsFailedTask[];
  childSummary: { total: number; completed: number; failed: number; running: number; settled: boolean };
}

/** `GET /api/tasks/{id}` for a task still queued: no execution row yet, from the live queue. */
export interface OpsQueuedTaskDetail {
  taskId: number;
  kind: string;
  pipeline: string;
  stage: string;
  folderPath: string;
  path: string;
  status: 'queued';
}

/**
 * One row of `activity.alert_acks`: whether a standing alert is acked, and
 * how many times its condition cleared (recurrence memory for escalation).
 */
export interface OpsAlertAck {
  class: string;
  key: string;
  acked: boolean;
  seen_clears: number;
  updated_at: string;
}

/**
 * `POST /api/transcripts/backfill`: either a fresh wave (`queued` with its
 * `taskId`) or the already-running one (`running`, no id to name).
 */
export interface OpsBackfillResult {
  ok: boolean;
  queued: boolean;
  taskId?: number;
  from?: string | null;
  running?: boolean;
}
