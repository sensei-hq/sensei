/**
 * Scan state classes — workspace state + activity state for the scan page,
 * plus the ScanState orchestrator singleton that owns the SSE subscription
 * and survives stage navigation.
 */

import { ReactiveStageContext } from './stage.svelte.js';
import { EventManager } from './events.js';
import { senseiApi, dataOrThrow, failureReason, type ApiResult } from './api.js';
import { appState } from './appstate.svelte.js';
import type { ScanWorkspace, ScanWorkspaceFolder, ScanFolderEvent, ActivityEvent, StateEvent } from './types.js';

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
   * `wizardState.scan.done`, set by the task-queue idle poller in the scan
   * page. Use this getter only for per-workspace UI ("3 of 5 workspaces ready"),
   * never to gate stage advancement.
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

// ── ScanState (orchestrator singleton) ──────────────────────────────────────

export type ScanRunStatus = 'idle' | 'starting' | 'scanning' | 'done' | 'error';

/**
 * Owns the live scan: SSE subscription, workspaces/activities state, and the
 * task-queue idle poller. Singleton-scoped because a scan runs for minutes
 * and the user can navigate between wizard stages while it runs. Component-
 * local state would tear down on unmount, lose the SSE stream, and leave
 * the user staring at "Begin scan" while the daemon is still indexing.
 *
 * Lifecycle:
 *   - reset() is called once per wizard session from WizardState.hydrate().
 *   - start() opens the SSE, kicks scans, starts the idle poller. Idempotent
 *     while already scanning.
 *   - SSE is held open until reset() — page mount/unmount does not touch it.
 */
export class ScanState {
  readonly workspaces = new ScanWorkspaceState();
  readonly activities = new ScanActivityState();

  status = $state<ScanRunStatus>('idle');
  rootCount = $state(0);
  error = $state<string | null>(null);
  daemonReachable = $state(true);

  #events: EventManager<StateEvent<{ id: string }>> | null = null;
  #unsub: (() => void) | null = null;
  #pollTimer: ReturnType<typeof setInterval> | null = null;
  #pollTimeout: ReturnType<typeof setTimeout> | null = null;

  get started(): boolean { return this.status !== 'idle'; }
  /**
   * True when the scan reached terminal idle (queue empty for two
   * consecutive polls). Exposed as both `isDone` (legacy) and `completed`
   * (new) — gates the wizard's Continue button via
   * `wizardState.canAdvance('scan')`.
   */
  get isDone(): boolean    { return this.status === 'done'; }
  get completed(): boolean { return this.status === 'done'; }

  /**
   * Engage the scan stage. Idempotent — calls while already in flight or
   * already done are no-ops. Safe to call from onMount so the page can
   * resume an in-flight scan after navigation without a manual button.
   *
   * Sequence (ordering is load-bearing):
   *   1. Open SSE and await the handshake. Without the await the
   *      subsequent scan POST can race the connection and the
   *      first wave of events fans out to zero subscribers.
   *   2. Hydrate already-known workspaces/folders from /api/workspaces.
   *      Covers the case where the scan finished (or partly finished)
   *      before this page was mounted.
   *   3. Trigger a scan of any roots still flagged unscanned.
   *   4. Start the queue-idle poller — the authoritative "done" signal.
   */
  async start(): Promise<void> {
    if (this.status === 'starting' || this.status === 'scanning' || this.status === 'done') return;

    const port = appState.port;
    const api = senseiApi(port);

    this.#teardown();
    this.workspaces.items = [];
    this.activities.items = [];
    this.error = null;
    this.daemonReachable = true;
    this.status = 'starting';

    try {
      // 1. Subscribe to SSE first so the broadcast channel has a listener
      //    before any new work emits. We don't await ready() — some
      //    webviews (notably WebKit) don't fire the `open` event until the
      //    first message arrives, which would deadlock when the queue is
      //    idle. Hydrate() below makes its own API roundtrip which gives
      //    the SSE handshake plenty of time to complete in practice.
      const events = new EventManager<StateEvent<{ id: string }>>(
        `http://127.0.0.1:${port}/api/scan/events`,
        (data) => JSON.parse(data) as StateEvent<{ id: string }>,
      );
      this.#unsub = events.subscribe((event) => this.#dispatch(event));
      this.#events = events;

      // 2. Hydrate from DB so a completed-or-partly-completed scan is
      //    visible immediately instead of waiting for an event that may
      //    never arrive (already happened).
      await this.#hydrate(api);

      this.status = 'scanning';

      // 3. Scan the roots that haven't produced any repos yet. A failed read
      //    (#288) or a refused scan (#304) throws into this.error — either
      //    left nothing scanning, and the poller would have called it done.
      const roots = dataOrThrow(await api.tryGetScanRoots(), 'roots');
      this.rootCount = roots.length;
      await triggerScans(api, roots);

      // 4. Done poller — long-running, marks status='done' on queue idle.
      this.#startDonePoller(api);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.status = 'error';
    }
  }

  /**
   * Pull current workspace + folder state from the daemon and seed the
   * reactive stores. Status flags default to 'indexed' for hydrated
   * folders — if a folder is in DB it has at least been registered, and
   * any in-flight folder will have its status corrected by the next
   * folder_update SSE event.
   */
  async #hydrate(api: ReturnType<typeof senseiApi>): Promise<void> {
    const workspaces = dataOrThrow(await api.tryListWorkspaces(), 'workspaces') as unknown as Array<{
      id: string;
      name: string;
      folders?: Array<{
        id: string;
        name: string;
        abs_path?: string;
        path?: string;
        kind?: string;
        status?: string;
        stack?: string[];
      }>;
    }>;
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
          stack: f.stack ?? [],
          filesTotal: 0,
          filesCompleted: 0,
          status: (f.status as ScanWorkspaceFolder['status']) ?? 'indexed',
        })),
      }));
    this.workspaces.set(items);
  }

  /**
   * Full teardown — closes SSE, clears state. Called from WizardState.hydrate()
   * so a fresh wizard session starts clean, and re-callable if the user wants
   * to abandon the current run.
   */
  reset(): void {
    this.#teardown();
    this.workspaces.items = [];
    this.activities.items = [];
    this.rootCount = 0;
    this.error = null;
    this.daemonReachable = true;
    this.status = 'idle';
  }

  #dispatch(event: StateEvent<{ id: string }>): void {
    if (event.entity === 'workspace')
      this.workspaces.apply(event as StateEvent<ScanWorkspace>);
    else if (event.entity === 'folder')
      this.workspaces.applyFolder(event as StateEvent<ScanFolderEvent>);
    else if (event.entity === 'activity')
      this.activities.apply(event as StateEvent<ActivityEvent>);
  }

  #teardown(): void {
    this.#unsub?.();
    this.#unsub = null;
    this.#events?.destroy();
    this.#events = null;
    if (this.#pollTimer) { clearInterval(this.#pollTimer); this.#pollTimer = null; }
    if (this.#pollTimeout) { clearTimeout(this.#pollTimeout); this.#pollTimeout = null; }
  }

  /**
   * Poll /api/index/status until the task queue is idle. SSE gives per-item
   * progress but no "queue idle" event, so polling is the authoritative
   * completion signal. Two consecutive idle polls debounce transient empty
   * queues between barrier tasks. No upper time limit — large codebases
   * legitimately take hours.
   */
  #startDonePoller(api: ReturnType<typeof senseiApi>): void {
    let idlePolls = 0;
    this.#pollTimeout = setTimeout(() => {
      this.#pollTimer = setInterval(async () => {
        const step = pollStep(await api.tryGetIndexStatus(), idlePolls);
        this.daemonReachable = step.reachable;
        idlePolls = step.idlePolls;
        if (step.done) {
          this.status = 'done';
          if (this.#pollTimer) { clearInterval(this.#pollTimer); this.#pollTimer = null; }
        }
      }, 1500);
    }, 1500);
  }
}

export const scanState = new ScanState();

type IndexStatus = { queue: { pending: number; running: number } };

/**
 * Start a scan of every root not yet scanned — in parallel, so a slow root does
 * not hold up its siblings. Throws naming each root the daemon would not scan
 * and why (#304: a refusal was ignored, and an idle queue then read as done).
 */
export async function triggerScans(
  api: { tryScanFolder: (root: string) => Promise<ApiResult<unknown>> },
  roots: Array<{ path: string; scanned: boolean }>,
): Promise<void> {
  const pending = roots.filter((r) => !r.scanned);
  const results = await Promise.all(pending.map(async (r) => ({ path: r.path, res: await api.tryScanFolder(r.path) })));
  const refused = results.flatMap(({ path, res }) => (res.ok ? [] : [`${path}: ${failureReason(res.error)}`]));
  if (refused.length > 0) throw new Error(`The scan did not start for ${refused.join('; ')}`);
}

/**
 * One tick of the scan's done poller. Two consecutive idle reads mean done; a
 * busy queue resets the count; a failed read is the daemon unreachable — never
 * idle, never done. (It used to fall back to an empty queue, so a daemon that
 * stopped answering mid-scan read as reachable AND idle, #288.)
 */
export function pollStep(
  read: ApiResult<IndexStatus>,
  idlePolls: number,
): { reachable: boolean; idlePolls: number; done: boolean } {
  if (!read.ok) return { reachable: false, idlePolls: 0, done: false };
  const idle = read.data.queue.pending === 0 && read.data.queue.running === 0;
  const count = idle ? idlePolls + 1 : 0;
  return { reachable: true, idlePolls: count, done: count >= 2 };
}
