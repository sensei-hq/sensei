// The scan client: the daemon's scan routes, by base URL (#496). The desktop
// app builds it from the daemon's port; the paths are pinned in
// client.spec.ts, so a route change is a loud one.
import { EventManager } from '../event-manager.js';
import { failureFrom, type ApiResult } from '../result.js';
import type { StreamFrame } from '../events.js';
import type {
  IndexQueueStatus,
  ScanDaemonRoot,
  ScanDaemonWorkspace,
} from './types.js';
import { parseScanStreamEvent } from './types.js';

// ─── The client ─────────────────────────────────────────────────────────────

export interface ScanApi {
  tryListWorkspaces: () => Promise<ApiResult<ScanDaemonWorkspace[]>>;
  tryGetScanRoots: () => Promise<ApiResult<ScanDaemonRoot[]>>;
  tryScanFolder: (root: string) => Promise<ApiResult<{ ok: boolean; scanning: boolean }>>;
  tryGetIndexStatus: () => Promise<ApiResult<IndexQueueStatus>>;
}

/** The scan client: every call the daemon's answer or an `ApiResult` error,
 *  the daemon's own refusal text when it sends one. */
export const scanClient = (base: string): ScanApi => {
  async function call<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method,
        ...(body === undefined ? {} : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
      });
      if (res.status === 204) return { ok: true, data: undefined as T };
      if (res.ok) return { ok: true, data: (await res.json()) as T };
      return { ok: false, error: await failureFrom(res) };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  return {
    tryListWorkspaces: () => call<ScanDaemonWorkspace[]>('GET', '/api/workspaces'),
    tryGetScanRoots: () => call<ScanDaemonRoot[]>('GET', '/api/scan/roots'),
    tryScanFolder: (root) =>
      call<{ ok: boolean; scanning: boolean }>('POST', '/api/scan', { root, max_depth: 3 }),
    tryGetIndexStatus: () => call<IndexQueueStatus>('GET', '/api/index/status'),
  };
};

// ─── Stream ─────────────────────────────────────────────────────────────────

export const scanEvents = (base: string): EventManager<StreamFrame> =>
  new EventManager<StreamFrame>(`${base}/api/scan/events`, parseScanStreamEvent);
