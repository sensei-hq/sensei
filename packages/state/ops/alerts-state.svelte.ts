import { classifyAlert, sortAlerts, type AlertClass, type OpsAlert } from './alerts.js';
import type { JobsState } from './jobs-state.svelte.js';
import { failingSchedules, type SchedulesState } from './schedules-state.svelte.js';
import type { SyncState } from './sync-state.svelte.js';
import { failureReason, type ApiResult } from '../result.js';
import type { OpsAlertAck } from './types.js';

// AlertsState derives standing alerts from the three domain states — never
// from its own fetch. Repeats dedupe by (class, key): one entry with a
// count, toasting once. A cleared condition drops its entry (and its ack);
// a recurrence escalates one level. The poller toasts new warnings and
// criticals; info-level refusals land in the section silently.
//
// Dispositions persist: acks and clears write through to the source, and a
// hydrate restores them, so reloads keep ack flags and recurrence memory.
// Derivation stays authoritative for STANDING — the source only annotates.

export interface AlertsDeps {
  jobs: JobsState;
  schedules: SchedulesState;
  sync: SyncState;
}

/** What AlertsState needs of its source: disposition reads and writes. */
export interface AlertsSource {
  readAcks: () => Promise<ApiResult<{ acks: OpsAlertAck[] }>>;
  writeAck: (cls: string, key: string, acked: boolean) => Promise<ApiResult<OpsAlertAck>>;
}

/** Cleared keys remembered for recurrence, FIFO-capped — a session's memory, not a log. */
const HISTORY_CAP = 500;

interface Draft {
  class: AlertClass;
  key: string;
  title: string;
  detail: string;
  fields: Parameters<typeof classifyAlert>[1];
}

function escalate(level: OpsAlert['level']): OpsAlert['level'] {
  return level === 'info' ? 'warn' : 'critical';
}

export class AlertsState {
  private readonly deps: AlertsDeps;
  private readonly source: AlertsSource;
  private history: string[] = [];
  private dispositions = new Map<string, OpsAlertAck>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private onAlert: ((alert: OpsAlert) => void) | null = null;

  alerts = $state<OpsAlert[]>([]);
  /** The dispositions read/write fault, when the source refuses — named, never silent. */
  dispositionsError = $state<string | null>(null);

  constructor(deps: AlertsDeps, source: AlertsSource) {
    this.deps = deps;
    this.source = source;
  }

  /**
   * Hydrates persisted dispositions: acked flags and recurrence memory from
   * before this session. Runs before the first derive (start does it);
   * re-runnable to pick up dispositions written elsewhere.
   */
  async loadDispositions(): Promise<void> {
    const read = await this.source.readAcks();
    if (!read.ok) {
      this.dispositionsError = `alert dispositions unreadable (${failureReason(read.error)})`;
      return;
    }
    this.dispositionsError = null;
    this.dispositions = new Map(read.data.acks.map((a) => [`${a.class}:${a.key}`, a]));
  }

  /**
   * Derives the current conditions and reconciles them with the standing
   * alerts. Returns the newly created ones (for toasts); repeats and
   * cleared entries are folded in silently.
   */
  refresh(): OpsAlert[] {
    const now = Date.now();
    const drafts = this.derive();
    const standing = new Map(this.alerts.map((a) => [`${a.class}:${a.key}`, a]));
    const created: OpsAlert[] = [];
    const next: OpsAlert[] = [];
    const emitted = new Set<string>();

    for (const draft of drafts) {
      const id = `${draft.class}:${draft.key}`;
      // One alert per condition: feeds can repeat a key (task ids restart
      // per session), and a repeated key would render twice and throw.
      // Feeds arrive newest-first, so first-wins keeps the newest.
      if (emitted.has(id)) continue;
      emitted.add(id);
      const prior = standing.get(id);
      if (prior) {
        prior.count += 1;
        prior.lastSeen = now;
        prior.title = draft.title;
        prior.detail = draft.detail;
        next.push(prior);
      } else {
        const severity = classifyAlert(draft.class, draft.fields);
        const disp = this.dispositions.get(id);
        // Recurrence is session memory OR durable memory: a key cleared
        // before this session escalates on sight, no re-learning needed.
        const recurred = this.history.includes(id) || (disp?.seen_clears ?? 0) > 0;
        const level = recurred ? escalate(severity.level) : severity.level;
        const alert: OpsAlert = {
          ...severity,
          level,
          class: draft.class,
          key: draft.key,
          title: draft.title,
          detail: draft.detail,
          count: 1,
          firstSeen: now,
          lastSeen: now,
          acked: disp?.acked ?? false,
          escalated: recurred,
        };
        created.push(alert);
        next.push(alert);
      }
      standing.delete(id);
    }
    // Whatever remains stood for a condition that cleared: drop it (ack with
    // it), remember the key, and write the clear through — the write is what
    // makes the recurrence memory survive this session.
    const cleared = [...standing.keys()];
    for (const id of cleared) {
      this.remember(id);
    }
    if (cleared.length > 0) {
      // flushClears faults by slot; this catch is for a source that throws
      // instead of answering — it faults the same slot, never silent.
      void this.flushClears(cleared).catch((e: unknown) => {
        this.dispositionsError = `alert dispositions unwritable (${e instanceof Error ? e.message : 'write threw'})`;
      });
    }
    this.alerts = sortAlerts(next);
    return created;
  }

  /**
   * Silences one standing alert until its condition clears or recurs. Writes
   * through first: a refused write leaves the alert unacked and returns the
   * reason, so the control can say so instead of lying.
   */
  async ack(class_: AlertClass, key: string): Promise<ApiResult<OpsAlertAck>> {
    const res = await this.source.writeAck(class_, key, true);
    if (!res.ok) {
      this.dispositionsError = `alert dispositions unwritable (${failureReason(res.error)})`;
      return res;
    }
    this.dispositionsError = null;
    this.dispositions.set(`${class_}:${key}`, res.data);
    const found = this.alerts.find((a) => a.class === class_ && a.key === key);
    if (found) found.acked = true;
    return res;
  }

  /** Records client-observed clears; a refusal faults the slot, never vanishes. */
  private async flushClears(ids: string[]): Promise<void> {
    for (const id of ids) {
      const sep = id.indexOf(':');
      const res = await this.source.writeAck(id.slice(0, sep), id.slice(sep + 1), false);
      if (!res.ok) {
        this.dispositionsError = `alert dispositions unwritable (${failureReason(res.error)})`;
        return;
      }
      this.dispositions.set(id, res.data);
    }
    this.dispositionsError = null;
  }

  /**
   * Polls the domains, toasting new warnings and criticals. Hydrates first:
   * the opening derive must see persisted acks, or a reloaded ack re-toasts.
   * Toasts immediately on start (a standing failure is news on arrival),
   * then every `intervalMs`. Info-level refusals never toast — they wait in
   * the section.
   */
  start(onAlert: (alert: OpsAlert) => void, intervalMs = 30000): void {
    this.stop();
    this.onAlert = onAlert;
    void (async () => {
      await this.loadDispositions();
      if (this.onAlert) this.toast(this.refresh());
    })();
    this.timer = setInterval(() => {
      if (this.onAlert) this.toast(this.refresh());
    }, intervalMs);
  }

  stop(): void {
    if (this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
    this.onAlert = null;
  }

  /** Full teardown — stops the poller, clears alerts and recurrence memory. */
  reset(): void {
    this.stop();
    this.alerts = [];
    this.history = [];
    this.dispositions = new Map();
    this.dispositionsError = null;
  }

  private toast(created: OpsAlert[]): void {
    for (const alert of created) {
      if (alert.level !== 'info') this.onAlert?.(alert);
    }
  }

  private remember(id: string): void {
    if (!this.history.includes(id)) {
      this.history.push(id);
      if (this.history.length > HISTORY_CAP) this.history.shift();
    }
  }

  private derive(): Draft[] {
    const { jobs, schedules, sync } = this.deps;
    const drafts: Draft[] = [];

    if (!jobs.daemonReachable || !schedules.daemonReachable || !sync.daemonReachable) {
      drafts.push({
        class: 'daemon',
        key: 'daemon',
        title: 'Daemon unreachable',
        detail: 'The center shows the last known state, not the current one.',
        fields: {},
      });
    }
    const faults: Array<[string, string | null]> = [
      ['jobs', jobs.streamFault],
      ['schedules', schedules.streamFault],
      ['sync', sync.streamFault],
    ];
    for (const [domain, fault] of faults) {
      if (fault) {
        drafts.push({
          class: 'stream',
          key: domain,
          title: `Live updates interrupted (${domain})`,
          detail: fault,
          fields: {},
        });
      }
    }
    for (const row of jobs.failed) {
      drafts.push({
        class: 'task',
        key: String(row.taskId),
        title: `${row.kind} failed`,
        detail: row.error ?? row.path,
        fields: { error: row.error },
      });
    }
    for (const row of jobs.abandoned?.worst ?? []) {
      drafts.push({
        class: 'task',
        key: `abandoned:${row.task_kind}:${row.path}`,
        title: `${row.task_kind} abandoned ${row.count}×`,
        detail: row.path,
        fields: { abandoned: true },
      });
    }
    for (const row of jobs.indexErrors) {
      drafts.push({
        class: 'task',
        key: `index:${row.folder_id}:${row.file_path}`,
        title: 'Index error',
        detail: `${row.file_path}: ${row.error}`,
        fields: { indexError: true },
      });
    }
    for (const row of failingSchedules(schedules.items)) {
      drafts.push({
        class: 'schedule',
        key: row.name,
        title: `${row.name} failing`,
        detail: row.last_error ?? 'last run failed',
        fields: {},
      });
    }
    for (const row of sync.rows) {
      if (row.state === 'error') {
        drafts.push({
          class: 'sync',
          key: row.entity_key,
          title: `${row.entity_key} sync error`,
          detail: [row.last_error, row.remedy].filter(Boolean).join(' — '),
          fields: {},
        });
      } else if (row.state === 'skipped' && row.remedy) {
        drafts.push({
          class: 'sync',
          key: row.entity_key,
          title: `${row.entity_key} sync skipped`,
          detail: [row.last_error, row.remedy].filter(Boolean).join(' — '),
          fields: { skipped: true, remedy: row.remedy },
        });
      }
    }
    for (const [persona, entry] of Object.entries(sync.activity)) {
      if (entry.state === 'failed') {
        drafts.push({
          class: 'sync',
          key: `pass:${persona}`,
          title: `${persona} ${entry.stage} failed`,
          detail: entry.error ?? 'stage failed',
          fields: {},
        });
      }
    }
    return drafts;
  }
}
