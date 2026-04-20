/**
 * RepoStore — reactive class for project/repo state.
 * Owns: projects, solutions, SSE subscription, search/filter.
 * Daemon owns exclusions. UI just renders what this class exposes.
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
  updatedAt: number;
}

export interface SolutionEntry {
  id: string;
  name: string;
  description: string;
  category: string;
  repos: RepoEntry[];
}

// ── Reactive Class ───────────────────────────────────────────────────────────

export class RepoStore {
  // State
  all = $state<RepoEntry[]>([]);
  solutions = $state<SolutionEntry[]>([]);
  search = $state('');

  // Derived: standalone projects (not in any solution)
  private solutionRepoIds = $derived(
    new Set(this.solutions.flatMap(s => s.repos.map(r => r.project.repo_id)))
  );

  standalone = $derived.by(() => {
    let list = this.all.filter(r => !this.solutionRepoIds.has(r.project.repo_id));
    if (this.search) {
      const q = this.search.toLowerCase();
      list = list.filter(r =>
        r.project.name.toLowerCase().includes(q) ||
        r.project.path.toLowerCase().includes(q)
      );
    }
    return this.sortRepos(list);
  });

  // Derived: solutions with their repos enriched from `all`
  enrichedSolutions = $derived.by(() => {
    return this.solutions.map(sol => ({
      ...sol,
      repos: sol.repos.map(sr => {
        const live = this.all.find(r => r.project.repo_id === sr.project.repo_id);
        return live ?? sr;
      }),
    })).filter(sol => {
      if (!this.search) return true;
      const q = this.search.toLowerCase();
      return sol.name.toLowerCase().includes(q) ||
        sol.repos.some(r => r.project.name.toLowerCase().includes(q));
    });
  });

  // Derived: flat list of all repos (solutions + standalone), sorted
  repos = $derived.by(() => {
    const fromSolutions = this.solutions.flatMap(s =>
      s.repos.map(sr => this.all.find(r => r.project.repo_id === sr.project.repo_id) ?? sr)
    );
    let list = [...fromSolutions, ...this.standalone.filter(r =>
      !this.solutionRepoIds.has(r.project.repo_id)
    )];
    if (this.search) {
      const q = this.search.toLowerCase();
      list = list.filter(r =>
        r.project.name.toLowerCase().includes(q) ||
        r.project.path.toLowerCase().includes(q)
      );
    }
    return this.sortRepos(list);
  });

  totalCount = $derived(this.all.length);
  indexedCount = $derived(this.all.filter(r => r.indexState === 'indexed').length);
  indexingCount = $derived(this.all.filter(r => r.indexState === 'indexing' || r.indexState === 'queued').length);
  failedCount = $derived(this.all.filter(r => r.indexState === 'failed').length);
  anyIndexing = $derived(this.all.some(r => r.indexState === 'indexing' || r.indexState === 'queued'));
  // Aggregate file counts — only repos with file-level tasks (not scan/repo meta-tasks)
  totalFiles = $derived(this.all.filter(r => r.filesTotal > 0).reduce((s, r) => s + r.filesTotal, 0));
  completedFiles = $derived(this.all.filter(r => r.filesTotal > 0).reduce((s, r) => s + r.filesCompleted, 0));
  solutionCount = $derived(this.solutions.length);

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
    this.es = new EventSource(`http://127.0.0.1:${this.port}/api/tasks/progress`);
    this.es.onmessage = (e) => { try { this.onEvent(JSON.parse(e.data)); } catch {} };
    this.es.onerror = () => { this.disconnect(); setTimeout(() => this.connect(), 3000); };
    this.fetchAll();
    this.pollTimer = setInterval(() => this.fetchAll(), 5000);
  }

  disconnect() {
    this.es?.close(); this.es = null;
    if (this.pollTimer) { clearInterval(this.pollTimer); this.pollTimer = null; }
  }

  // ── Actions ────────────────────────────────────────────────────────────────

  async scanFolder(path: string) { await senseiApi(this.port).scanFolder(path); }

  async indexRepo(repoId: string, repoPath: string, force = false) {
    await senseiApi(this.port).indexRepo(repoId, repoPath, force);
  }

  async excludeRepo(repoId: string) {
    await senseiApi(this.port).excludeRepo(repoId);
    this.all = this.all.filter(r => r.project.repo_id !== repoId);
  }

  getRepo(repoId: string): RepoEntry | undefined {
    return this.all.find(r => r.project.repo_id === repoId);
  }

  getSolution(solutionId: string): SolutionEntry | undefined {
    return this.solutions.find(s => s.id === solutionId);
  }

  // ── Internal ───────────────────────────────────────────────────────────────

  private sortRepos(list: RepoEntry[]): RepoEntry[] {
    return [...list].sort((a, b) => {
      const aActive = a.indexState === 'indexing' || a.indexState === 'queued' ? 0 : 1;
      const bActive = b.indexState === 'indexing' || b.indexState === 'queued' ? 0 : 1;
      if (aActive !== bActive) return aActive - bActive;
      return b.updatedAt - a.updatedAt;
    });
  }

  private async fetchAll() {
    const api = senseiApi(this.port);
    try {
      const [projects, status, rawSolutions] = await Promise.all([
        api.getProjects() as Promise<ServerProject[]>,
        api.getIndexStatus() as Promise<any>,
        api.listSolutions() as Promise<any[]>,
      ]);

      const repoStatus: Record<string, any> = status.repos ?? {};
      const now = Date.now();

      // Build repo entries
      this.all = projects.map(p => this.buildEntry(p, repoStatus, now));

      // Build solution entries with repo references
      this.solutions = (rawSolutions ?? []).map((sol: any) => ({
        id: sol.id,
        name: sol.name,
        description: sol.description ?? '',
        category: sol.category ?? 'active',
        repos: (sol.repos ?? []).map((sr: any) => {
          const repoId = sr.repo_id ?? sr.repoId;
          const existing = this.all.find(r => r.project.repo_id === repoId);
          if (existing) return existing;
          // Repo in solution but not in projects list (maybe excluded or not yet scanned)
          return {
            project: { repo_id: repoId, name: sr.label ?? repoId, path: sr.path ?? '', stack: [], libs: [], tags: [], status: 'unknown', repoId, indexedAt: undefined, lastError: undefined, partiallyIndexed: false } as ServerProject,
            indexState: 'idle' as IndexState,
            filesTotal: 0, filesCompleted: 0, filesFailed: 0, currentFile: null, updatedAt: now,
          };
        }),
      }));
    } catch { /* daemon unavailable */ }
  }

  private buildEntry(p: ServerProject, repoStatus: Record<string, any>, now: number): RepoEntry {
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

    const prevState = existing?.indexState;
    const updatedAt = (prevState && prevState === indexState) ? (existing?.updatedAt ?? now) : now;

    return { project: p, indexState, filesTotal, filesCompleted, filesFailed, currentFile, updatedAt };
  }

  private onEvent(evt: { event: string; repo_id?: string; path?: string }) {
    if (!evt.repo_id) return;
    const idx = this.all.findIndex(r => r.project.repo_id === evt.repo_id);

    if (idx === -1) {
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
