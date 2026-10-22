import { failureReason, type ApiResult } from './api.js';

/** A workspace a cross-workspace screen could not read, and why. */
export interface UnreadWorkspace { name: string; reason: string }

/**
 * Run one read per workspace, all at once. Each read that lands is kept with its
 * workspace; each that fails is named in `unread` with its reason — so the screen
 * shows what it has AND says what it is missing, rather than letting a failed
 * workspace pass as a clean one (#288).
 */
export async function fanOutWorkspaces<P extends { name: string }, T>(
  workspaces: readonly P[],
  read: (workspace: P) => Promise<ApiResult<T>>,
): Promise<{ read: Array<{ workspace: P; data: T }>; unread: UnreadWorkspace[] }> {
  const results = await Promise.all(workspaces.map(async (workspace) => ({ workspace, r: await read(workspace) })));
  return {
    read: results.flatMap(({ workspace, r }) => (r.ok ? [{ workspace, data: r.data }] : [])),
    unread: results.flatMap(({ workspace, r }) => (r.ok ? [] : [{ name: workspace.name, reason: failureReason(r.error) }])),
  };
}
