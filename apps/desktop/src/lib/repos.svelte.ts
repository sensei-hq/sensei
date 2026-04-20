/**
 * RepoStore — reactive class for project/repo state.
 * Simple: fetch list, subscribe to SSE, update in place, sort by activity.
 * Daemon owns exclusions — this class just renders what the daemon says exists.
 */
import { senseiApi } from './api.js';
import type { ServerProject } from './types.js';

// ── Types ────────────────────────────────────────────────────────────────────

export type IndexState = 'idle' | 'queued' | 'indexing' | 'indexed' | 'failed';

export interface RepoEntry {
  project: ServerProject;
  indexState: IndexState;
  filesTotal: number;
  filesCompleted: number;
  filesFailed: number;
  currentFile: string | null;
  updatedAt: number; // epoch ms — for recency sort
}

// ── Reactive Class ───────────────────────────────────────────────────────────

export class RepoStore {
  // State
  all = $state<RepoEntry[]>([]);
  search = $state('');

  // Derived
  repos = $derived.by(() => {
    let list = this.all;
    if (this.search) {
      const q = this.search.toLowerCase();
      list = list.filter(r =>
        r.project.name.toLowerCase().includes(q) ||
        r.project.path.toLowerCase().includes(q)
      );
    }
    return [...list].sort((a, b) => {
      // In-progress first
      const aActive = a.indexState === 'indexing' || a.indexState === 'queued' ? 0 : 1;
      const bActive = b.indexState === 'indexing' || b.indexState === 'queued' ? 0 : 1;
      if (aActive !== bActive) return aActive - bActive;
      // Then by recency (most recently updated first)
      return b.updatedAt - a.updatedAt;
    });
  });

  totalCount = $derived(this.all.length);
  indexedCount = $derived(this.all.filter(r => r.indexState === 'indexed').length);
  indexingCount = $derived(this.all.filter(r => r.indexState === 'indexing' || r.indexState === 'queued').length);
  failedCount = $derived(this.all.filter(r => r.indexState === 'failed').length);
  anyIndexing = $derived(this.all.some(r => r.indexState === 'indexing' || r.indexState === 'queued'));

  // Aggregate totals derived from individual repos
  totalFiles = $derived(this.all.reduce((s, r) => s + r.filesTotal, 0));
  completedFiles = $derived(this.all.reduce((s, r) => s + r.filesCompleted, 0));

  // Private
  private port: number;
  private es: EventSource | null = null;
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  constructor(port: number) {
    this.port = port;
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  connect() {
    this.disconnect();

    // Subscribe to SSE first — don't miss events during initial fetch
    this.es = new EventSource(`http://127.0.0.1:${this.port}/api/tasks/progress`);
    this.es.onmessage = (e) => { try { this.onEvent(JSON.parse(e.data)); } catch {} };
    this.es.onerror = () => { this.disconnect(); setTimeout(() => this.connect(), 3000); };

    // Then fetch current state
    this.fetchAll();

    // Periodic full sync as backup
    this.pollTimer = setInterval(() => this.fetchAll(), 5000);
  }

  disconnect() {
    this.es?.close(); this.es = null;
    if (this.pollTimer) { clearInterval(this.pollTimer); this.pollTimer = null; }
  }

  // ── Actions ────────────────────────────────────────────────────────────────

  async scanFolder(path: string) {
    await senseiApi(this.port).scanFolder(path);
  }

  async indexRepo(repoId: string, repoPath: string, force = false) {
    await senseiApi(this.port).indexRepo(repoId, repoPath, force);
  }

  async excludeRepo(repoId: string) {
    await senseiApi(this.port).excludeRepo(repoId);
    // Remove from local list immediately (daemon already excluded it)
    this.all = this.all.filter(r => r.project.repo_id !== repoId);
  }

  // ── Fetch (full sync from daemon) ──────────────────────────────────────────

  private async fetchAll() {
    const api = senseiApi(this.port);
    try {
      const [projects, status] = await Promise.all([
        api.getProjects() as Promise<ServerProject[]>,
        api.getIndexStatus() as Promise<any>,
      ]);

      const repoStatus: Record<string, any> = status.repos ?? {};
      const now = Date.now();

      this.all = projects.map(p => {
        const rs = repoStatus[p.repo_id];
        const existing = this.all.find(r => r.project.repo_id === p.repo_id);

        let indexState: IndexState = 'idle';
        let filesTotal = 0, filesCompleted = 0, filesFailed = 0;
        let currentFile: string | null = null;

        if (rs) {
          filesTotal = rs.total ?? 0;
          filesCompleted = rs.completed ?? 0;
          filesFailed = rs.failed ?? 0;
          currentFile = rs.current_file ?? null;

          if (rs.running > 0) indexState = 'indexing';
          else if (rs.pending > 0) indexState = 'queued';
          else if (filesFailed > 0 && filesCompleted === 0) indexState = 'failed';
          else if (filesCompleted > 0 && rs.pending === 0 && rs.running === 0) indexState = 'indexed';
        }

        if (indexState === 'idle' && p.indexed_at) indexState = 'indexed';

        // Preserve updatedAt if state hasn't changed, otherwise bump
        const prevState = existing?.indexState;
        const updatedAt = (prevState && prevState === indexState) ? (existing?.updatedAt ?? now) : now;

        return { project: p, indexState, filesTotal, filesCompleted, filesFailed, currentFile, updatedAt };
      });
    } catch { /* daemon unavailable */ }
  }

  // ── SSE event handling ─────────────────────────────────────────────────────

  private onEvent(evt: { event: string; repo_id?: string; path?: string }) {
    if (!evt.repo_id) return;
    const idx = this.all.findIndex(r => r.project.repo_id === evt.repo_id);

    if (idx === -1) {
      // New repo appeared (from scan) — fetch all to pick it up
      this.fetchAll();
      return;
    }

    const entry = { ...this.all[idx] };
    const now = Date.now();

    if (evt.event === 'started') {
      entry.indexState = 'indexing';
      if (evt.path) entry.currentFile = evt.path;
      entry.updatedAt = now;
    } else if (evt.event === 'completed') {
      entry.filesCompleted++;
      entry.currentFile = null;
      entry.updatedAt = now;
      // Check if all done
      if (entry.filesCompleted + entry.filesFailed >= entry.filesTotal && entry.filesTotal > 0) {
        entry.indexState = 'indexed';
      }
    } else if (evt.event === 'failed') {
      entry.filesFailed++;
      entry.currentFile = null;
      entry.updatedAt = now;
    }

    const updated = [...this.all];
    updated[idx] = entry;
    this.all = updated;
  }
}

// ── Singleton ────────────────────────────────────────────────────────────────

let _instance: RepoStore | null = null;

export function getRepoStore(port = 7744): RepoStore {
  if (!_instance) _instance = new RepoStore(port);
  return _instance;
}
