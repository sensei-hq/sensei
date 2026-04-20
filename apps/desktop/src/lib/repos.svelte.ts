/**
 * Repos state class — single source of truth for all project/repo data.
 * Subscribes to SSE, polls daemon API, exposes reactive lists.
 * UI just renders what this class exposes.
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

// ── State ────────────────────────────────────────────────────────────────────

let _repos = $state<Map<string, RepoState>>(new Map());
let _queueTotals = $state<{ pending: number; running: number; completed: number; failed: number }>({ pending: 0, running: 0, completed: 0, failed: 0 });
let _searchQuery = $state('');
let _filterState = $state<RepoState['indexState'] | 'all'>('all');
let _excluded = $state<Set<string>>(new Set());
let _eventSource: EventSource | null = null;
let _port = 7744;
let _pollTimer: ReturnType<typeof setInterval> | null = null;
let _refreshDebounce: ReturnType<typeof setTimeout> | null = null;
let _onChange: (() => void) | null = null;

// ── Accessors (reactive — UI derives from these) ────────────────────────────

// ── Search / Filter / Exclude ────────────────────────────────────────────────

export function setSearch(query: string) { _searchQuery = query; }
export function getSearch(): string { return _searchQuery; }

export function setFilter(state: RepoState['indexState'] | 'all') { _filterState = state; }
export function getFilter() { return _filterState; }

export function exclude(repoId: string) { _excluded = new Set([..._excluded, repoId]); }
export function include(repoId: string) { const s = new Set(_excluded); s.delete(repoId); _excluded = s; }
export function getExcluded(): string[] { return [..._excluded]; }
export function isExcluded(repoId: string): boolean { return _excluded.has(repoId); }

// ── Derived (reactive — Svelte auto-tracks dependencies) ────────────────────

/** All repos unfiltered */
export const allRepos = $derived([..._repos.values()]);

/** Filtered + searched + sorted repos — UI renders this */
export const repos = $derived.by(() => {
  let list = [..._repos.values()];

  // Exclude
  if (_excluded.size > 0) {
    list = list.filter(r => !_excluded.has(r.project.repo_id));
  }

  // Filter by state
  if (_filterState !== 'all') {
    list = list.filter(r => r.indexState === _filterState);
  }

  // Search
  if (_searchQuery) {
    const q = _searchQuery.toLowerCase();
    list = list.filter(r =>
      r.project.name.toLowerCase().includes(q) ||
      r.project.path.toLowerCase().includes(q) ||
      (r.project.stack ?? []).some((s: string) => s.toLowerCase().includes(q))
    );
  }

  // Sort: indexing first, then by name
  list.sort((a, b) => {
    const aActive = a.indexState === 'indexing' || a.indexState === 'queued' ? 0 : 1;
    const bActive = b.indexState === 'indexing' || b.indexState === 'queued' ? 0 : 1;
    if (aActive !== bActive) return aActive - bActive;
    return a.project.name.localeCompare(b.project.name);
  });

  return list;
});

export const queueTotals = $derived(_queueTotals);
export const indexedCount = $derived([..._repos.values()].filter(r => r.indexState === 'indexed').length);
export const totalCount = $derived(_repos.size);
export const anyIndexing = $derived([..._repos.values()].some(r => r.indexState === 'indexing' || r.indexState === 'queued'));
export const searchQuery = $derived(_searchQuery);
export const filterState = $derived(_filterState);
export const excludedList = $derived([..._excluded]);

export function getRepo(repoId: string): RepoState | undefined {
  return _repos.get(repoId);
}

export function onReposChange(cb: () => void) { _onChange = cb; }
export function offReposChange() { _onChange = null; }

// ── Actions ──────────────────────────────────────────────────────────────────

export async function scanFolder(path: string) {
  const api = senseiApi(_port);
  await api.scanFolder(path);
  scheduleRefresh();
}

export async function indexRepo(repoId: string, repoPath: string, force = false) {
  const api = senseiApi(_port);
  await api.indexRepo(repoId, repoPath, force);
  scheduleRefresh();
}

export async function indexAll(force = false) {
  for (const [, repo] of _repos) {
    if (repo.indexState !== 'indexing' && repo.indexState !== 'queued') {
      await indexRepo(repo.project.repo_id, repo.project.path, force);
    }
  }
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

export function connect(port: number) {
  _port = port;
  disconnect();

  // SSE subscription
  _eventSource = new EventSource(`http://127.0.0.1:${port}/api/tasks/progress`);

  _eventSource.onmessage = (event) => {
    try {
      const data: SSEEvent = JSON.parse(event.data);
      applyEvent(data);
    } catch { /* ignore parse errors */ }
    scheduleRefresh();
  };

  _eventSource.onerror = () => {
    disconnect();
    setTimeout(() => connect(_port), 3000);
  };

  // Periodic full sync (backup for missed SSE events)
  _pollTimer = setInterval(() => fullRefresh(), 5000);

  // Initial load
  fullRefresh();
}

export function disconnect() {
  _eventSource?.close();
  _eventSource = null;
  if (_pollTimer) { clearInterval(_pollTimer); _pollTimer = null; }
  if (_refreshDebounce) { clearTimeout(_refreshDebounce); _refreshDebounce = null; }
}

// ── Internal ─────────────────────────────────────────────────────────────────

function scheduleRefresh() {
  if (_refreshDebounce) clearTimeout(_refreshDebounce);
  _refreshDebounce = setTimeout(() => fullRefresh(), 300);
}

async function fullRefresh() {
  const api = senseiApi(_port);

  try {
    const [projects, status] = await Promise.all([
      api.getProjects() as Promise<ServerProject[]>,
      api.getIndexStatus() as Promise<QueueStatus>,
    ]);

    const newRepos = new Map<string, RepoState>();

    for (const p of projects) {
      const repoId = p.repo_id;
      const progress = status.repos[repoId];
      const progressState: RepoProgress | null = progress ? {
        total: progress.total,
        pending: progress.pending,
        running: progress.running,
        completed: progress.completed ?? 0,
        failed: progress.failed ?? 0,
        currentFile: progress.current_file ?? null,
      } : null;

      let indexState: RepoState['indexState'] = 'idle';
      if (progressState && (progressState.running > 0)) {
        indexState = 'indexing';
      } else if (progressState && progressState.pending > 0) {
        indexState = 'queued';
      } else if (progressState && progressState.failed > 0 && progressState.completed === 0) {
        indexState = 'failed';
      } else if (p.indexed_at || (progressState && progressState.completed > 0 && progressState.pending === 0 && progressState.running === 0)) {
        indexState = 'indexed';
      }

      newRepos.set(repoId, { project: p, progress: progressState, indexState });
    }

    // Trigger Svelte reactivity
    _repos = newRepos;

    _queueTotals = {
      pending: status.queue.pending,
      running: status.queue.running,
      completed: status.queue.completed,
      failed: 0,
    };

    _onChange?.();
  } catch { /* daemon unavailable */ }
}

function applyEvent(evt: SSEEvent) {
  if (!evt.repo_id) return;

  const existing = _repos.get(evt.repo_id);
  if (!existing) return; // project not yet registered

  const progress = existing.progress ?? { total: 0, pending: 0, running: 0, completed: 0, failed: 0, currentFile: null };

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

  // Trigger reactivity with new Map
  const updated = new Map(_repos);
  updated.set(evt.repo_id, { ...existing, progress: { ...progress }, indexState });
  _repos = updated;
}
