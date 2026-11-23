/**
 * Scan state — workspace state + activity state for scan progress, plus the
 * ScanState orchestrator that owns the SSE subscription and survives
 * navigation. Moved from the app (#496): a state never fetches, so it is
 * built with a source, and the app holds one module-level instance every
 * progress surface shares.
 */

import { ReactiveStageContext } from '../stage.svelte.js';
import type { StateEvent, StreamFrame } from '../events.js';
import type { StateStream } from '../source.js';
import { failureReason, type ApiResult } from '../result.js';
import type {
  ActivityEvent,
  IndexQueueStatus,
  QueueCounts,
  ScanDaemonRoot,
  ScanDaemonWorkspace,
  ScanFolderEvent,
  ScanWorkspace,
  ScanWorkspaceFolder,
} from './types.js';

/** What ScanState needs of its source: the reads, the trigger, and the stream. */
export interface ScanSource {
  readWorkspaces: () => Promise<ApiResult<ScanDaemonWorkspace[]>>;
  readRoots: () => Promise<ApiResult<ScanDaemonRoot[]>>;
  triggerScan: (root: string) => Promise<ApiResult<unknown>>;
  readIndexStatus: () => Promise<ApiResult<IndexQueueStatus>>;
  events: StateStream<StreamFrame>;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Compute common parent path from an array of absolute paths. */
export function commonParent(paths: string[]): string {
  if (paths.length === 0) return '';
  if (paths.length === 1) return paths[0].substring(0, paths[0].lastIndexOf('/'));

  const parts = paths.map(p => p.split('/'));
  const common: string[] = [];

  for (let i = 0; i < parts[0].length; i++) {
    const segment = parts[0][i];
    if (parts.every(p => p[i] === segment)) {
      common.push(segment);
    } else {
      break;
    }
  }

  return common.join('/') || '/';
}

function dataOrThrow<T>(read: ApiResult<T>, what: string): T {
  if (!read.ok) throw new Error(`${what}: ${failureReason(read.error)}`);
  return read.data;
}

// ── ScanWorkspaceState ─────────────────────────────────────────────────────────

export class ScanWorkspaceState extends ReactiveStageContext<ScanWorkspace> {

  /**
   * Override update to merge folders by id rather than replacing the whole array.
   * Daemon sends partial folder updates — only changed folders in the array.
   */
  override update(data: ScanWorkspace | Partial<ScanWorkspace> | (ScanWorkspace | Partial<ScanWorkspace>)[]) {
    const patches = Array.isArray(data) ? data : [data];
    let changed = false;

    for (const patch of patches) {
      if (!patch.id) continue;
      const idx = this.items.findIndex(p => p.id === patch.id);
      if (idx < 0) continue;

      const existing = this.items[idx];
      // Filter out empty-string / undefined / empty-array fields from patch so the
      // daemon can emit partial updates (e.g., status flips only) without nulling
      // out the existing name, folders, etc.
      const cleaned: Partial<ScanWorkspace> = {};
      for (const [k, v] of Object.entries(patch)) {
        if (v === '' || v === undefined) continue;
        if (Array.isArray(v) && v.length === 0) continue;
        (cleaned as any)[k] = v;
      }
      const merged = { ...existing, ...cleaned };

      // Merge folders by id if patch contains folders
      if (patch.folders && existing.folders) {
        const folderMap = new Map(existing.folders.map(f => [f.id, f]));
        for (const pf of patch.folders) {
          folderMap.set(pf.id, { ...(folderMap.get(pf.id) ?? pf as ScanWorkspaceFolder), ...pf });
        }
        merged.folders = Array.from(folderMap.values());
      }

      this.items[idx] = merged as ScanWorkspace;
      changed = true;
    }

    if (changed) this.items = [...this.items];
  }

  /**
   * Apply a folder-entity SSE event. The daemon sends folder events with a
   * workspaceId field for routing. On add: find the workspace and add/merge the
   * folder, or create a placeholder workspace if the workspace isn't in state yet
   * (happens on re-scans where workspace_add is not re-emitted). On update:
   * delegate to the existing folder-merge logic in update().
   */
  applyFolder(event: StateEvent<ScanFolderEvent>) {
    const folders = Array.isArray(event.data) ? event.data : [event.data];
    const now = Date.now();
    for (const folder of folders) {
      const { workspaceId, ...rest } = folder;
      const folderData = { ...rest, lastUpdated: now };
      const proj = this.items.find(p => p.id === workspaceId);

      if (event.action === 'add') {
        if (proj) {
          if (!proj.folders.find(f => f.id === folder.id)) {
            this.items = this.items.map(p =>
              p.id === workspaceId ? { ...p, folders: [...p.folders, folderData] } : p
            );
          }
        } else {
          // Workspace not yet in state (re-scan of existing workspace) — create placeholder
          this.add({
            id: workspaceId,
            name: folder.name,
            status: 'scanning',
            folders: [folderData],
            autoDetected: true,
            confidence: 'high',
          } as ScanWorkspace);
        }
      } else if (event.action === 'update') {
        // Reuse the folder-merge logic already in update()
        this.update({ id: workspaceId, folders: [folderData] } as Partial<ScanWorkspace>);
      }
    }
  }

  // ── Derived ──────────────────────────────────────────────

  /** Derive workspace display path from its folders. */
  workspacePath(workspace: ScanWorkspace): string {
    return commonParent(workspace.folders.map(f => f.path));
  }

  /** One workspace's scan counts — the rule the totals below sum (audit
   *  2026-10-28, item 15: the scan page recounted these in its template). A
   *  folder is ready once 'indexed'. */
  progressOf(workspace: ScanWorkspace): { folders: number; ready: number; files: number; completed: number } {
    return {
      folders: workspace.folders.length,
      ready: workspace.folders.filter(f => f.status === 'indexed').length,
      files: workspace.folders.reduce((n, f) => n + f.filesTotal, 0),
      completed: workspace.folders.reduce((n, f) => n + f.filesCompleted, 0),
    };
  }

  #sum(key: 'folders' | 'ready' | 'files' | 'completed'): number {
    return this.items.reduce((n, p) => n + this.progressOf(p)[key], 0);
  }

  get totalFolders() {
    return this.#sum('folders');
  }

  get readyFolders() {
    return this.#sum('ready');
  }

  /**
   * Folders found but not yet finished — every status that is not the
   * terminal 'indexed'. Includes discovered/queued/indexing/failed so that
   * pending + ready == totalFolders, giving the UI a stat that inversely
   * correlates with PROCESSED as work progresses.
   */
  get pendingFolders() {
    return this.totalFolders - this.readyFolders;
  }

  /**
   * Flattened folder list across all workspaces, sorted by lastUpdated DESC.
   * Drives the per-task panel on the scan stage — folders are keyed by id
   * so status transitions show through, unlike activity events which are
   * append-only. Folders without lastUpdated (hydrated from DB, never
   * touched by SSE) sort to the bottom.
   */
  get allFolders(): ScanWorkspaceFolder[] {
    return this.items.flatMap(p => p.folders).sort((a, b) =>
      (b.lastUpdated ?? 0) - (a.lastUpdated ?? 0)
    );
  }

  /**
   * Workspaces sorted by their newest folder's lastUpdated DESC, so the
   * workspace card with the most recent activity (a fresh folder_update from
   * the daemon) bubbles to the top. Workspaces whose folders have no
   * lastUpdated (hydrated, no SSE traffic) sort to the bottom in their
   * original order.
   */
  get sortedItems(): ScanWorkspace[] {
    const recency = (p: ScanWorkspace) =>
      p.folders.reduce((m, f) => (f.lastUpdated && f.lastUpdated > m ? f.lastUpdated : m), 0);
    return [...this.items].sort((a, b) => recency(b) - recency(a));
  }

  get totalFiles() {
    return this.#sum('files');
  }

  get completedFiles() {
    return this.#sum('completed');
  }

  get scanning() {
    return this.items.some(p => p.status === 'scanning' || p.status === 'indexing');
  }

  /**
   * Every workspace has reached a terminal state (active or failed).
   *
   * NOTE: This is NOT the canonical "scan complete" signal — that lives on
   * the run status, set by the task-queue idle poller. Use this getter only
   * for per-workspace UI ("3 of 5 workspaces ready"), never to gate stage
   * advancement.
   */
  get allWorkspacesResolved() {
    return this.items.length > 0 && this.items.every(p => p.status === 'active' || p.status === 'failed');
  }
}

// ── ScanActivityState ────────────────────────────────────────────────────────

export class ScanActivityState extends ReactiveStageContext<ActivityEvent> {

  /**
   * Recent events, newest first.
   *
   * Layout is newest-at-top: each new event makes older ones shift DOWN, not
   * up. This is intentional — the live cursor stays anchored where the user
   * is reading. Don't "fix" the order to append-bottom without also flipping
   * the container's scroll anchor.
   */
  get recent() {
    return this.items.slice(-100).reverse();
  }

  /**
   * Maximum elapsed value across received events. Using max rather than
   * "latest" keeps the displayed time monotonically increasing — each daemon
   * handler times from its own start (scan_root is scan-relative and grows;
   * process_git_folder reports small per-folder durations), so "latest" could
   * jump backwards. Max tracks the leading (scan-relative) clock.
   */
  get totalElapsed() {
    return this.items.reduce((max, e) => (e.elapsed > max ? e.elapsed : max), 0);
  }

  /** Stats bar counts derived from activity levels. */
  get discovered() {
    return this.items.filter(e => e.level === 'discover').length;
  }

  get queued() {
    return this.items.filter(e => e.level === 'queue').length;
  }

  get scanComplete() {
    return this.items.some(e => e.level === 'success');
  }
}

// ── ScanState (orchestrator) ──────────────────────────────────────────────

export type ScanRunStatus = 'idle' | 'starting' | 'scanning' | 'done' | 'error';

/** Queue poll cadence: fast while a run is live, slow while passively idle —
 *  the footer watches forever, but an idle daemon earns no chatter. */
const POLL_FAST_MS = 1500;
const POLL_SLOW_MS = 15000;

/**
 * Owns the live scan: source subscription, workspace/activity state, the
 * queue snapshot, and the queue-idle poller. Singleton-scoped in the app
 * because a scan runs for minutes and the user can navigate while it runs.
 * Component-local state would tear down on unmount, lose the stream, and
 * leave the user staring at stale progress while the daemon keeps indexing.
 *
 * Two modes share one subscription:
 *   - attach() observes passively (the footer): never triggers scans, and
 *     oscillates `idle ↔ scanning` on queue busyness. It never writes
 *     `done` — done gates the wizard Continue button, and a footer must not
 *     trip it.
 *   - start() owns a run (the setup scan page): attaches, then triggers
 *     unscanned roots, and reaches `done` (or `error`) as today.
 *
 * Lifecycle:
 *   - reset() is called once per wizard session; it clears the run but keeps
 *     the footer's `expanded` UI state.
 *   - attach() is idempotent — calls while attached are no-ops. Safe to call
 *     from a mount effect so any surface can observe without coordinating.
 */
export class ScanState {
  readonly source: ScanSource;
  readonly workspaces = new ScanWorkspaceState();
  readonly activities = new ScanActivityState();

  status = $state<ScanRunStatus>('idle');
  rootCount = $state(0);
  error = $state<string | null>(null);
  daemonReachable = $state(true);
  /** Latest queue counts from the poller — the mini band's numbers. */
  queueSnapshot = $state<QueueCounts | null>(null);
  /** Mini ↔ band: the footer's own UI state, session-scoped. */
  expanded = $state(false);

  #unsub: (() => void) | null = null;
  #pollTimeout: ReturnType<typeof setTimeout> | null = null;
  #attached = false;
  #owning = false;
  #hydrated = false;
  #openedOnce = false;
  // Joinable in-flight work: the footer effect and the setup page attach
  // simultaneously on boot, so concurrent callers join one run instead of
  // racing past the status guard and double-subscribing (or dropping the
  // setup trigger). `#gen` retires a run a reset() overtook.
  #attaching: Promise<void> | null = null;
  #starting: Promise<void> | null = null;
  #gen = 0;

  constructor(source: ScanSource) {
    this.source = source;
  }

  get started(): boolean { return this.status !== 'idle'; }
  /**
   * True when the OWNED run reached terminal idle (queue empty for two
   * consecutive polls). Exposed as both `isDone` (legacy) and `completed`
   * (new) — gates the wizard's Continue button. Passive observation never
   * sets it.
   */
  get isDone(): boolean    { return this.status === 'done'; }
  get completed(): boolean { return this.status === 'done'; }

  /**
   * Observe the daemon's indexing without triggering anything. Idempotent:
   * calls while attached are no-ops, so every surface can attach from its
   * mount effect without coordinating.
   *
   * Sequence (ordering is load-bearing):
   *   1. Subscribe first so the broadcast channel has a listener before any
   *      work emits. The initial open is NOT awaited — some webviews
   *      (notably WebKit) don't fire `open` until the first message arrives,
   *      which would deadlock when the queue is idle.
   *   2. Hydrate already-known workspaces from the source's read. Covers a
   *      scan that finished (or partly finished) before this surface
   *      mounted. A reconnect open re-hydrates (broadcast has no replay).
   *   3. Read the queue once: busy reads as `scanning`, idle stays `idle`.
   *   4. Start the poller — the authoritative activity signal, and the
   *      recovery path when the daemon was down at attach time.
   */
  attach(): Promise<void> {
    if (this.#attached) return Promise.resolve();
    if (this.#attaching) return this.#attaching;
    this.#attaching = this.#doAttach().finally(() => {
      this.#attaching = null;
    });
    return this.#attaching;
  }

  async #doAttach(): Promise<void> {
    const gen = this.#gen;
    this.#teardown();
    this.workspaces.set([]);
    this.activities.set([]);
    this.error = null;
    this.daemonReachable = true;
    this.queueSnapshot = null;
    this.status = 'starting';

    // 1. Subscribe first; the first open is the initial subscription, not a
    //    reconnect, so the handler skips it (attach hydrates below). A socket
    //    fault reads as unreachable at once — the poller corrects it if the
    //    daemon still answers, so the fault never outlives the outage.
    this.#unsub = this.source.events(
      (event) => this.#dispatch(event),
      () => {
        if (!this.#openedOnce) {
          this.#openedOnce = true;
          return;
        }
        void this.#hydrate().catch(() => {
          // A reconnect read that fails must read as a fault, not stale
          // stores: unreachable until the poller says otherwise (D83).
          this.daemonReachable = false;
        });
      },
      () => {
        this.daemonReachable = false;
      },
    );
    this.#attached = true;

    try {
      // 2. Hydrate from the DB so a completed-or-partly-completed scan is
      //    visible immediately instead of waiting for an event that may
      //    never arrive (already happened).
      await this.#hydrate();
      if (gen !== this.#gen) return;

      // 3. The queue decides the passive status — observed, never owned.
      const first = await this.source.readIndexStatus();
      if (gen !== this.#gen) return;
      this.daemonReachable = first.ok;
      if (first.ok) {
        const { pending, running, completed } = first.data.queue;
        this.queueSnapshot = { pending, running, completed };
        this.status = pending === 0 && running === 0 ? 'idle' : 'scanning';
      } else {
        this.status = 'idle';
      }
    } catch {
      // A daemon that is down at attach time reads as unreachable and idle,
      // never as failed: the poller below is the recovery path.
      this.daemonReachable = false;
      this.status = 'idle';
    }
    this.#ensurePoller();
  }

  /**
   * Own a scan run: attach, then trigger every unscanned root. Idempotent
   * while an owned run is in flight or done; upgrades a passive observation
   * (the footer's) to an owned run without resubscribing. A refused trigger
   * fails the run and stops the poller — an idle queue over a scan that
   * never started must not read as done (#304).
   */
  start(): Promise<void> {
    if (this.status === 'done') return Promise.resolve();
    if (this.status === 'scanning' && this.#owning && !this.#starting) return Promise.resolve();
    if (this.#starting) return this.#starting;
    this.#starting = this.#doStart().finally(() => {
      this.#starting = null;
    });
    return this.#starting;
  }

  async #doStart(): Promise<void> {
    const gen = this.#gen;
    this.#owning = true;
    await this.attach();
    if (gen !== this.#gen) return;
    this.status = 'scanning';

    try {
      const roots = dataOrThrow(await this.source.readRoots(), 'roots');
      this.rootCount = roots.length;
      await triggerScans(this.source, roots);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.status = 'error';
      this.#stopPoller();
      return;
    }
    this.#ensurePoller();
  }

  /**
   * Full teardown — closes the stream, clears the run. Called once per
   * wizard session so a fresh wizard starts clean, and re-callable to
   * abandon the current run. `expanded` survives: it is the footer's UI
   * state, not the run's.
   */
  reset(): void {
    this.#gen += 1;
    this.#teardown();
    this.workspaces.set([]);
    this.activities.set([]);
    this.rootCount = 0;
    this.error = null;
    this.daemonReachable = true;
    this.queueSnapshot = null;
    this.status = 'idle';
    this.#attached = false;
    this.#owning = false;
    this.#hydrated = false;
    this.#openedOnce = false;
  }

  /**
   * Pull current workspace + folder state from the source and seed the
   * reactive stores. Status flags default to 'indexed' for hydrated
   * folders — if a folder is in DB it has at least been registered, and
   * any in-flight folder will have its status corrected by the next
   * folder_update event.
   */
  async #hydrate(): Promise<void> {
    const workspaces = dataOrThrow(await this.source.readWorkspaces(), 'workspaces');
    const items: ScanWorkspace[] = workspaces
      .filter((p) => (p.folders ?? []).length > 0)
      .map((p) => ({
        id: p.id,
        name: p.name,
        status: 'active' as const,
        autoDetected: true,
        confidence: 'high' as const,
        folders: (p.folders ?? []).map((f) => ({
          id: f.id,
          name: f.name,
          path: f.abs_path ?? f.path ?? '',
          kind: f.kind,
          stack: f.stack ?? [],
          filesTotal: 0,
          filesCompleted: 0,
          status: (f.status as ScanWorkspaceFolder['status']) ?? 'indexed',
        })),
      }));
    this.workspaces.set(items);
    this.#hydrated = true;
  }

  #dispatch(event: StreamFrame): void {
    if (event.entity === 'workspace')
      this.workspaces.apply(event as unknown as StateEvent<ScanWorkspace>);
    else if (event.entity === 'folder')
      this.workspaces.applyFolder(event as unknown as StateEvent<ScanFolderEvent>);
    else if (event.entity === 'activity')
      this.activities.apply(event as unknown as StateEvent<ActivityEvent>);
    // Folder/activity traffic means indexing is happening NOW — flip idle to
    // scanning at once, or a burst that drains between the slow idle polls
    // never shows (9.1). Idle only: never revives an error, never writes
    // done — the poller's two-idle rule still brings it back down.
    if (
      (event.entity === 'folder' || event.entity === 'activity') &&
      this.status === 'idle'
    ) {
      this.status = 'scanning';
    }
  }

  #teardown(): void {
    this.#unsub?.();
    this.#unsub = null;
    this.#stopPoller();
  }

  #stopPoller(): void {
    if (this.#pollTimeout) {
      clearTimeout(this.#pollTimeout);
      this.#pollTimeout = null;
    }
  }

  #ensurePoller(): void {
    if (this.#pollTimeout) return;
    let idlePolls = 0;
    const tick = async (): Promise<void> => {
      this.#pollTimeout = null;
      const read = await this.source.readIndexStatus();
      const step = pollStep(read, idlePolls);
      this.daemonReachable = step.reachable;
      idlePolls = step.idlePolls;
      if (read.ok) {
        const { pending, running, completed } = read.data.queue;
        this.queueSnapshot = { pending, running, completed };
        if (!this.#hydrated) {
          // Recovery: the attach-time read failed, so the stores are empty.
          // A failure here reads as unreachable and retries next tick (D83).
          await this.#hydrate().catch(() => {
            this.daemonReachable = false;
          });
        }
        if (pending === 0 && running === 0) {
          if (step.done) {
            // Two consecutive idle polls end an OWNED run; a passive
            // observation just goes quiet — and keeps watching.
            this.status = this.#owning ? 'done' : 'idle';
            if (this.#owning) return;
          }
        } else {
          this.status = 'scanning';
        }
      }
      // A timeout chain, not an interval, so the cadence follows the state:
      // fast while a run is live, slow while passively idle.
      const slow = !this.#owning && this.status === 'idle' && idlePolls >= 2;
      this.#pollTimeout = setTimeout(() => void tick(), slow ? POLL_SLOW_MS : POLL_FAST_MS);
    };
    this.#pollTimeout = setTimeout(() => void tick(), POLL_FAST_MS);
  }
}

/**
 * Start a scan of every root not yet scanned — in parallel, so a slow root does
 * not hold up its siblings. Throws naming each root the daemon would not scan
 * and why (#304: a refusal was ignored, and an idle queue then read as done).
 */
export async function triggerScans(
  api: { triggerScan: (root: string) => Promise<ApiResult<unknown>> },
  roots: Array<{ path: string; scanned: boolean }>,
): Promise<void> {
  const pending = roots.filter((r) => !r.scanned);
  const results = await Promise.all(pending.map(async (r) => ({ path: r.path, res: await api.triggerScan(r.path) })));
  const refused = results.flatMap(({ path, res }) => (res.ok ? [] : [`${path}: ${failureReason(res.error)}`]));
  if (refused.length > 0) throw new Error(`The scan did not start for ${refused.join('; ')}`);
}

/**
 * One tick of the scan's queue poller. Two consecutive idle reads mean done; a
 * busy queue resets the count; a failed read is the daemon unreachable — never
 * idle, never done. (It used to fall back to an empty queue, so a daemon that
 * stopped answering mid-scan read as reachable AND idle, #288.)
 */
export function pollStep(
  read: ApiResult<IndexQueueStatus>,
  idlePolls: number,
): { reachable: boolean; idlePolls: number; done: boolean } {
  if (!read.ok) return { reachable: false, idlePolls: 0, done: false };
  const idle = read.data.queue.pending === 0 && read.data.queue.running === 0;
  const count = idle ? idlePolls + 1 : 0;
  return { reachable: true, idlePolls: count, done: count >= 2 };
}

/**
 * The mini band's one line: file counts when the stream has delivered any,
 * queue counts when the poller has read any. "Queued" is pending + running
 * — everything still in the queue, whatever its phase. No snapshot, no
 * queue claim: unknown is not zero (D83).
 */
export function miniSummary(
  snapshot: QueueCounts | null,
  filesCompleted: number,
  filesTotal: number,
): string {
  if (snapshot === null) {
    if (filesTotal > 0) {
      return `Indexing ${filesCompleted.toLocaleString()}/${filesTotal.toLocaleString()} files`;
    }
    return 'Indexing…';
  }
  const queued = snapshot.pending + snapshot.running;
  if (filesTotal > 0) {
    return `Indexing ${filesCompleted.toLocaleString()}/${filesTotal.toLocaleString()} files · ${queued} queued`;
  }
  if (queued > 0) return `Indexing · ${queued} queued`;
  return 'Indexing…';
}

