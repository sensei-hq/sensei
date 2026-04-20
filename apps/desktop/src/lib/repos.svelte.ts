/**
 * RepoStore — reactive class managing all project/repo state.
 * Subscribes to SSE, polls daemon, exposes reactive derived lists.
 * UI just reads properties — all logic lives here.
 */
import { senseiApi } from './api.js';
import type { ServerProject } from './types.js';

// ── Types ────────────────────────────────────────────────────────────────────

export interface RepoState {
  project: ServerProject;
  progress: RepoProgress | null;
  indexState: 'idle' | 'queued' | 'indexing' | 'indexed' | 'failed';
}

export interface RepoProgress {
  total: number;
  pending: number;
  running: number;
  completed: number;
  failed: number;
  currentFile: string | null;
}

interface QueueStatus {
  queue: { pending: number; blocked: number; running: number; completed: number; repos_active?: number };
  repos: Record<string, { total: number; pending: number; running: number; completed?: number; failed?: number; current_file?: string | null }>;
}

interface SSEEvent {
  event: 'queued' | 'started' | 'completed' | 'failed';
  task_id: number;
  repo_id?: string;
  kind?: string;
  path?: string;
  error?: string;
}

// ── Reactive Class ───────────────────────────────────────────────────────────

export class RepoStore {
  // Reactive state
  all = $state<RepoState[]>([]);
  search = $state('');
  filter = $state<RepoState['indexState'] | 'all'>('all');
  excluded = $state<Set<string>>(new Set());
  queueTotals = $state({ pending: 0, running: 0, completed: 0, failed: 0 });

  // Derived (auto-recompute when dependencies change)
  repos = $derived.by(() => {
    let list = this.all;
    if (this.excluded.size > 0) list = list.filter(r => !this.excluded.has(r.project.repo_id));
    if (this.filter !== 'all') list = list.filter(r => r.indexState === this.filter);
    if (this.search) {
      const q = this.search.toLowerCase();
      list = list.filter(r =>
        r.project.name.toLowerCase().includes(q) ||
        r.project.path.toLowerCase().includes(q) ||
        (r.project.stack ?? []).some((s: string) => s.toLowerCase().includes(q))
      );
    }
    return [...list].sort((a, b) => {
      const aActive = a.indexState === 'indexing' || a.indexState === 'queued' ? 0 : 1;
      const bActive = b.indexState === 'indexing' || b.indexState === 'queued' ? 0 : 1;
      if (aActive !== bActive) return aActive - bActive;
      return a.project.name.localeCompare(b.project.name);
    });
  });

  indexedCount = $derived(this.all.filter(r => r.indexState === 'indexed').length);
  totalCount = $derived(this.all.length);
  anyIndexing = $derived(this.all.some(r => r.indexState === 'indexing' || r.indexState === 'queued'));

  // Private
  private port: number;
  private eventSource: EventSource | null = null;
  private pollTimer: ReturnType<typeof setInterval> | null = null;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(port: number) {
    this.port = port;
  }

  // ── Lifecycle ──────────────────────────────────────────────────────────────

  connect() {
    this.disconnect();

    this.eventSource = new EventSource(`http://127.0.0.1:${this.port}/api/tasks/progress`);

    this.eventSource.onmessage = (event) => {
      try {
        this.applyEvent(JSON.parse(event.data));
      } catch { /* ignore */ }
      this.scheduleRefresh();
    };

    this.eventSource.onerror = () => {
      this.disconnect();
      setTimeout(() => this.connect(), 3000);
    };

    this.pollTimer = setInterval(() => this.refresh(), 5000);
    this.refresh();
  }

  disconnect() {
    this.eventSource?.close();
    this.eventSource = null;
    if (this.pollTimer) { clearInterval(this.pollTimer); this.pollTimer = null; }
    if (this.debounceTimer) { clearTimeout(this.debounceTimer); this.debounceTimer = null; }
  }

  // ── Actions ────────────────────────────────────────────────────────────────

  async scanFolder(path: string) {
    await senseiApi(this.port).scanFolder(path);
    this.scheduleRefresh();
  }

  async indexRepo(repoId: string, repoPath: string, force = false) {
    await senseiApi(this.port).indexRepo(repoId, repoPath, force);
    this.scheduleRefresh();
  }

  async indexAll(force = false) {
    for (const repo of this.all) {
      if (repo.indexState !== 'indexing' && repo.indexState !== 'queued') {
        await this.indexRepo(repo.project.repo_id, repo.project.path, force);
      }
    }
  }

  exclude(repoId: string) { this.excluded = new Set([...this.excluded, repoId]); }
  include(repoId: string) { const s = new Set(this.excluded); s.delete(repoId); this.excluded = s; }

  getRepo(repoId: string): RepoState | undefined {
    return this.all.find(r => r.project.repo_id === repoId);
  }

  // ── Internal ───────────────────────────────────────────────────────────────

  private scheduleRefresh() {
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.debounceTimer = setTimeout(() => this.refresh(), 300);
  }

  private async refresh() {
    const api = senseiApi(this.port);
    try {
      const [projects, status] = await Promise.all([
        api.getProjects() as Promise<ServerProject[]>,
        api.getIndexStatus() as Promise<QueueStatus>,
      ]);

      this.all = projects.map(p => {
        const prog = status.repos[p.repo_id];
        const progress: RepoProgress | null = prog ? {
          total: prog.total, pending: prog.pending, running: prog.running,
          completed: prog.completed ?? 0, failed: prog.failed ?? 0,
          currentFile: prog.current_file ?? null,
        } : null;

        let indexState: RepoState['indexState'] = 'idle';
        if (progress && progress.running > 0) indexState = 'indexing';
        else if (progress && progress.pending > 0) indexState = 'queued';
        else if (progress && progress.failed > 0 && progress.completed === 0) indexState = 'failed';
        else if (p.indexed_at || (progress && progress.completed > 0 && progress.pending === 0 && progress.running === 0)) indexState = 'indexed';

        return { project: p, progress, indexState };
      });

      this.queueTotals = {
        pending: status.queue.pending, running: status.queue.running,
        completed: status.queue.completed, failed: 0,
      };
    } catch { /* daemon unavailable */ }
  }

  private applyEvent(evt: SSEEvent) {
    if (!evt.repo_id) return;
    const idx = this.all.findIndex(r => r.project.repo_id === evt.repo_id);
    if (idx === -1) return;

    const existing = this.all[idx];
    const progress = existing.progress
      ? { ...existing.progress }
      : { total: 0, pending: 0, running: 0, completed: 0, failed: 0, currentFile: null };

    if (evt.event === 'started') {
      progress.running++;
      if (progress.pending > 0) progress.pending--;
      if (evt.path) progress.currentFile = evt.path;
    } else if (evt.event === 'completed') {
      progress.completed++;
      if (progress.running > 0) progress.running--;
      progress.currentFile = null;
    } else if (evt.event === 'failed') {
      progress.failed++;
      if (progress.running > 0) progress.running--;
    }

    let indexState = existing.indexState;
    if (progress.running > 0) indexState = 'indexing';
    else if (progress.pending > 0) indexState = 'queued';
    else if (progress.completed > 0 && progress.pending === 0 && progress.running === 0) indexState = 'indexed';

    // Reassign array to trigger reactivity
    const updated = [...this.all];
    updated[idx] = { ...existing, progress, indexState };
    this.all = updated;
  }
}

// ── Singleton ────────────────────────────────────────────────────────────────
// Create once, import everywhere. Connect in root layout.

let _instance: RepoStore | null = null;

export function getRepoStore(port = 7744): RepoStore {
  if (!_instance) _instance = new RepoStore(port);
  return _instance;
}
