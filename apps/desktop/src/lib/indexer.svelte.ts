import type { IndexQueueStatus, IndexProgressEvent, DirtyStatus } from './types.js';
import { senseiApi } from './api.js';

// ── Reactive state ───────────────────────────────────────────────────────────

let _queueStatus = $state<IndexQueueStatus>({ current: null, queued: [], recent: [] });
let _progress = $state<Map<string, IndexProgressEvent>>(new Map());
let _dirty = $state<DirtyStatus[]>([]);
let _eventSource: EventSource | null = null;
let _port = 7744;
let _reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let _statusPollTimer: ReturnType<typeof setInterval> | null = null;
let _onChange: (() => void) | null = null;
let _pendingProgress = new Map<string, IndexProgressEvent>(); // buffer before flushing to reactive state
let _flushTimer: ReturnType<typeof setTimeout> | null = null;

// ── Accessors ────────────────────────────────────────────────────────────────

export function getQueueStatus(): IndexQueueStatus { return _queueStatus; }
export function getProgressForRepo(repoId: string): IndexProgressEvent | undefined { return _progress.get(repoId); }
export function getProgressMap(): Map<string, IndexProgressEvent> { return _progress; }
export function getDirtyFiles(): DirtyStatus[] { return _dirty; }
export function isConnected(): boolean { return _eventSource?.readyState === EventSource.OPEN; }

/** Register a callback for when indexing state changes (started, completed, failed, queued). */
export function onIndexChange(cb: () => void) { _onChange = cb; }
export function offIndexChange() { _onChange = null; }

export function isIndexing(repoId: string): boolean {
  // If SSE says completed/failed, it's done — regardless of stale queue status
  const p = _progress.get(repoId);
  if (p?.type === 'completed' || p?.type === 'failed') return false;
  // Check queue status
  if (_queueStatus.current?.repo_id === repoId) return true;
  if (_queueStatus.queued.some(j => j.repo_id === repoId)) return true;
  // Check SSE progress
  return p?.type === 'progress' || p?.type === 'started' || p?.type === 'queued';
}

export function getQueuePosition(repoId: string): number {
  const idx = _queueStatus.queued.findIndex(j => j.repo_id === repoId);
  return idx >= 0 ? idx + 1 : -1;
}

// ── SSE Connection ───────────────────────────────────────────────────────────

export function connectSSE(port: number) {
  _port = port;
  disconnectSSE();

  const api = senseiApi(port);
  _eventSource = api.subscribeIndexProgress();

  _eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data) as IndexProgressEvent;
      const repoId = data.repo_id;
      if (!repoId) return;

      if (data.type === 'progress') {
        // Buffer progress events — flush to reactive state periodically
        _pendingProgress.set(repoId, data);
        if (!_flushTimer) {
          _flushTimer = setTimeout(() => {
            _flushTimer = null;
            const next = new Map(_progress);
            for (const [k, v] of _pendingProgress) {
              // Don't overwrite completed/failed with stale progress
              const existing = next.get(k);
              if (existing?.type === 'completed' || existing?.type === 'failed') continue;
              next.set(k, v);
            }
            _pendingProgress.clear();
            _progress = next;
          }, 300);
        }
      } else {
        // Lifecycle events: update immediately and clear pending for this repo
        _pendingProgress.delete(repoId);
        _progress = new Map(_progress).set(repoId, data);
        refreshStatus();
        _onChange?.();
      }
    } catch { /* ignore parse errors */ }
  };

  _eventSource.onerror = () => {
    // Auto-reconnect after 2s (handles both CLOSED and CONNECTING states)
    if (_reconnectTimer) clearTimeout(_reconnectTimer);
    _reconnectTimer = setTimeout(() => {
      _reconnectTimer = null;
      if (!_eventSource || _eventSource.readyState !== EventSource.OPEN) {
        connectSSE(_port);
      }
    }, 2000);
  };

  // Poll queue status every 2s as backup (SSE can miss events between repos)
  if (_statusPollTimer) clearInterval(_statusPollTimer);
  _statusPollTimer = setInterval(() => {
    refreshStatus();
  }, 2000);

  // Initial load
  refreshStatus();
}

export function disconnectSSE() {
  _eventSource?.close();
  _eventSource = null;
  if (_reconnectTimer) { clearTimeout(_reconnectTimer); _reconnectTimer = null; }
  if (_statusPollTimer) { clearInterval(_statusPollTimer); _statusPollTimer = null; }
}

// ── Actions ──────────────────────────────────────────────────────────────────

export async function startIndex(repoId: string, repoPath: string, force = false) {
  const api = senseiApi(_port);
  const result = await api.indexRepo(repoId, repoPath, force);
  if (result.queued) {
    _progress = new Map(_progress).set(repoId, { type: 'queued', repo_id: repoId, position: result.position });
    refreshStatus();
  }
  return result;
}

export async function refreshStatus() {
  const api = senseiApi(_port);
  _queueStatus = await api.getIndexStatus();
}

export async function refreshDirty() {
  const api = senseiApi(_port);
  _dirty = await api.getIndexDirty();
}
