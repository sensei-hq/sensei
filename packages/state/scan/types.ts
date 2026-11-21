// The scan domain's wire shapes (#496): what the daemon's scan routes return
// and what its scan stream emits. Moved from the app's types.ts so every
// plane reads the same shapes.
import type { StateEvent } from '../events.js';

export type FolderStatus = 'discovered' | 'queued' | 'indexing' | 'indexed' | 'failed';
export type WorkspaceStatus = 'scanning' | 'indexing' | 'active' | 'failed';
export type ActivityLevel = 'discover' | 'queue' | 'process' | 'info' | 'success' | 'error';

export interface ScanWorkspaceFolder {
  id: string;
  name: string;
  path: string;
  stack: string[];
  filesTotal: number;
  filesCompleted: number;
  status: FolderStatus;
  /**
   * Wall-clock ms timestamp of the last add/update event the client received
   * for this folder. Drives the task-list panel's reverse-chronological
   * ordering — in-flight folders bubble up as their progress events arrive,
   * terminal-status folders drift down once their lastUpdated freezes.
   * Optional because hydration from /api/workspaces has no event timestamp.
   */
  lastUpdated?: number;
}

/** Folder-level SSE event from daemon — includes workspaceId for routing. */
export interface ScanFolderEvent extends ScanWorkspaceFolder {
  workspaceId: string;
}

export interface ScanWorkspace {
  id: string;
  name: string;
  status: WorkspaceStatus;
  folders: ScanWorkspaceFolder[];
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
}

export interface ActivityEvent {
  id: string;
  level: ActivityLevel;
  message: string;
  elapsed: number;
  timestamp: number;
}

/** One event on the scan stream: a workspace, folder, or activity change. */
export type ScanStreamEvent = StateEvent<{ id: string }>;

/** One daemon workspace row as /api/workspaces returns it (hydrate input). */
export interface ScanDaemonWorkspace {
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
}

/** One watch root as /api/scan/roots returns it (trigger input). */
export interface ScanDaemonRoot {
  path: string;
  scanned: boolean;
}

/** The queue counts the footer and the done poller share. */
export interface QueueCounts {
  pending: number;
  running: number;
  completed: number;
}

/** /api/index/status, as far as the scan state reads it. */
export interface IndexQueueStatus {
  queue: QueueCounts & { blocked: number };
}
