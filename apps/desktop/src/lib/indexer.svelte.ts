/**
 * Task queue reactive state — replaces old IndexQueue-based indexer.
 * Connects to /api/tasks/progress SSE for real-time updates.
 */
import { senseiApi } from './api.js';

// ── Types ────────────────────────────────────────────────────────────────────

interface TaskQueueStatus {
  queue: { pending: number; blocked: number; running: number; completed: number; repos_active?: number };
  repos: Record<string, { total: number; pending: number; running: number; completed?: number; failed?: number; current_file?: string }>;
}

// ── State ────────────────────────────────────────────────────────────────────

let _status = $state<TaskQueueStatus>({
  queue: { pending: 0, blocked: 0, running: 0, completed: 0, repos_active: 0 },
  repos: {},
});
let _eventSource: EventSource | null = null;
let _port = 7744;
let _onChange: (() => void) | null = null;
let _statusPollTimer: ReturnType<typeof setInterval> | null = null;

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

export function getQueuePosition(_repoId: string): number {
  return -1;
}

// Backward compat stubs
export function getProgressForRepo(_repoId: string): any { return undefined; }
export function getProgressMap(): Map<string, any> { return new Map(); }
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

  _eventSource.onmessage = () => {
    refreshStatus();
    _onChange?.();
  };

  _eventSource.onerror = () => {
    disconnectSSE();
    setTimeout(() => connectSSE(_port), 3000);
  };

  _statusPollTimer = setInterval(() => refreshStatus(), 5000);
  refreshStatus();
}

export function disconnectSSE() {
  _eventSource?.close();
  _eventSource = null;
  if (_statusPollTimer) {
    clearInterval(_statusPollTimer);
    _statusPollTimer = null;
  }
}

export async function refreshStatus() {
  const api = senseiApi(_port);
  try {
    _status = await api.getIndexStatus();
  } catch { /* ignore */ }
}
