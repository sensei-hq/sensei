/**
 * Task queue reactive state — connects to /api/tasks/progress SSE for real-time updates.
 * Parses SSE events (which include repo_id) and maintains per-repo progress.
 */
import { senseiApi } from './api.js';

// ── Types ────────────────────────────────────────────────────────────────────

interface RepoProgress {
  total: number;
  pending: number;
  running: number;
  completed?: number;
  failed?: number;
  current_file?: string | null;
  // Legacy fields from /all page
  type?: string;
  files_processed?: number;
  files_total?: number;
}

interface TaskQueueStatus {
  queue: { pending: number; blocked: number; running: number; completed: number; repos_active?: number };
  repos: Record<string, RepoProgress>;
}

interface TaskEvent {
  event: 'queued' | 'started' | 'completed' | 'failed';
  task_id: number;
  repo_id?: string;
  kind?: string;
  path?: string;
  error?: string;
}

// ── State ────────────────────────────────────────────────────────────────────

let _status = $state<TaskQueueStatus>({
  queue: { pending: 0, blocked: 0, running: 0, completed: 0 },
  repos: {},
});
let _eventSource: EventSource | null = null;
let _port = 7744;
let _onChange: (() => void) | null = null;
let _statusPollTimer: ReturnType<typeof setInterval> | null = null;
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;

// ── Accessors ────────────────────────────────────────────────────────────────

export function getQueueStatus(): TaskQueueStatus { return _status; }
export function isConnected(): boolean { return _eventSource?.readyState === EventSource.OPEN; }

export function onIndexChange(cb: () => void) { _onChange = cb; }
export function offIndexChange() { _onChange = null; }

export function isIndexing(repoId: string): boolean {
  const repo = _status.repos[repoId];
  if (!repo) return false;
  return repo.running > 0 || repo.pending > 0;
}

export function getRepoProgress(repoId: string): RepoProgress | undefined {
  return _status.repos[repoId];
}

export function getQueuePosition(_repoId: string): number {
  return -1;
}

// Legacy compat
export function getProgressForRepo(repoId: string): any { return _status.repos[repoId]; }
export function getProgressMap(): Map<string, RepoProgress> {
  return new Map(Object.entries(_status.repos));
}
export function getDirtyFiles(): any[] { return []; }

// ── Actions ──────────────────────────────────────────────────────────────────

export async function startIndex(repoId: string, repoPath: string, force = false) {
  const api = senseiApi(_port);
  await api.indexRepo(repoId, repoPath, force);
  refreshStatus();
  _onChange?.();
}

// ── SSE Connection ───────────────────────────────────────────────────────────

export function connectSSE(port: number) {
  _port = port;
  disconnectSSE();

  _eventSource = new EventSource(`http://127.0.0.1:${port}/api/tasks/progress`);

  _eventSource.onmessage = (event) => {
    // Parse SSE event data — contains repo_id for per-repo tracking
    try {
      const data: TaskEvent = JSON.parse(event.data);
      handleEvent(data);
    } catch { /* ignore parse errors */ }

    // Debounced full refresh + notify callback
    if (_debounceTimer) clearTimeout(_debounceTimer);
    _debounceTimer = setTimeout(() => {
      refreshStatus();
      _onChange?.();
    }, 300);
  };

  _eventSource.onerror = () => {
    disconnectSSE();
    setTimeout(() => connectSSE(_port), 3000);
  };

  // Periodic full refresh as backup (in case SSE events are missed)
  _statusPollTimer = setInterval(() => {
    refreshStatus();
    _onChange?.();
  }, 5000);

  refreshStatus();
}

function handleEvent(evt: TaskEvent) {
  // Update local state immediately for responsiveness (full refresh follows via debounce)
  if (evt.event === 'started' && evt.repo_id) {
    const repo = _status.repos[evt.repo_id] ?? { total: 0, pending: 0, running: 0, completed: 0, failed: 0, current_file: null };
    repo.running++;
    if (repo.pending > 0) repo.pending--;
    if (evt.path) repo.current_file = evt.path;
    // Trigger reactivity by reassigning
    _status = { ..._status, repos: { ..._status.repos, [evt.repo_id]: { ...repo } } };
  } else if (evt.event === 'completed' && evt.repo_id) {
    const repo = _status.repos[evt.repo_id] ?? { total: 0, pending: 0, running: 0, completed: 0, failed: 0, current_file: null };
    repo.completed = (repo.completed ?? 0) + 1;
    if (repo.running > 0) repo.running--;
    repo.current_file = null;
    _status = { ..._status, repos: { ..._status.repos, [evt.repo_id]: { ...repo } } };
  } else if (evt.event === 'failed' && evt.repo_id) {
    const repo = _status.repos[evt.repo_id] ?? { total: 0, pending: 0, running: 0, completed: 0, failed: 0, current_file: null };
    repo.failed = (repo.failed ?? 0) + 1;
    if (repo.running > 0) repo.running--;
    _status = { ..._status, repos: { ..._status.repos, [evt.repo_id]: { ...repo } } };
  }
}

export function disconnectSSE() {
  _eventSource?.close();
  _eventSource = null;
  if (_statusPollTimer) {
    clearInterval(_statusPollTimer);
    _statusPollTimer = null;
  }
  if (_debounceTimer) {
    clearTimeout(_debounceTimer);
    _debounceTimer = null;
  }
}

export async function refreshStatus() {
  const api = senseiApi(_port);
  try {
    const fresh = await api.getIndexStatus();
    // Force Svelte reactivity by creating a new object
    _status = { queue: { ...fresh.queue }, repos: { ...fresh.repos } };
  } catch { /* ignore */ }
}
