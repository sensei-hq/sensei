import type { IndexQueueStatus, IndexProgressEvent, DirtyStatus } from './types.js';
import { senseiApi } from './api.js';

// ── Reactive state ───────────────────────────────────────────────────────────

let _queueStatus = $state<IndexQueueStatus>({ current: null, queued: [], recent: [] });
let _progress = $state<Map<string, IndexProgressEvent>>(new Map());
let _dirty = $state<DirtyStatus[]>([]);
let _eventSource: EventSource | null = null;
let _port = 7744;

// ── Accessors ────────────────────────────────────────────────────────────────

export function getQueueStatus(): IndexQueueStatus { return _queueStatus; }
export function getProgressForRepo(repoId: string): IndexProgressEvent | undefined { return _progress.get(repoId); }
export function getDirtyFiles(): DirtyStatus[] { return _dirty; }
export function isConnected(): boolean { return _eventSource?.readyState === EventSource.OPEN; }

export function isIndexing(repoId: string): boolean {
  if (_queueStatus.current?.repo_id === repoId) return true;
  const p = _progress.get(repoId);
  return p?.type === 'progress' || p?.type === 'started';
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
      if (repoId) {
        _progress = new Map(_progress).set(repoId, data);
      }

      // On completed/failed, refresh queue status
      if (data.type === 'completed' || data.type === 'failed') {
        refreshStatus();
      }
    } catch { /* ignore parse errors */ }
  };

  _eventSource.onerror = () => {
    // Auto-reconnect after 3s
    setTimeout(() => {
      if (_eventSource?.readyState === EventSource.CLOSED) {
        connectSSE(_port);
      }
    }, 3000);
  };

  // Initial load
  refreshStatus();
  refreshDirty();
}

export function disconnectSSE() {
  _eventSource?.close();
  _eventSource = null;
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
