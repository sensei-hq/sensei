import { invoke } from '@tauri-apps/api/core';

/**
 * Open (or focus) a dedicated Tauri window for a project.
 *
 * Idempotent — a subsequent call for the same `projectId` focuses the
 * existing window rather than opening a duplicate. The window loads
 * `/project/{projectId}/overview` so the sidebar shell hydrates against
 * the routed project directly.
 *
 * Throws when the Tauri IPC bridge is unavailable (running in a browser)
 * or when the daemon rejects the id — callers can surface the error.
 */
export async function openProjectWindow(projectId: string): Promise<void> {
  if (!projectId) {
    throw new Error('projectId required');
  }
  await invoke('open_project_window', { projectId });
}
