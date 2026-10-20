import { failureReason, type ApiResult } from './api.js';

/** A project a cross-project screen could not read, and why. */
export interface UnreadProject { name: string; reason: string }

/**
 * Run one read per project, all at once. Each read that lands is kept with its
 * project; each that fails is named in `unread` with its reason — so the screen
 * shows what it has AND says what it is missing, rather than letting a failed
 * project pass as a clean one (#288).
 */
export async function fanOutProjects<P extends { name: string }, T>(
  projects: readonly P[],
  read: (project: P) => Promise<ApiResult<T>>,
): Promise<{ read: Array<{ project: P; data: T }>; unread: UnreadProject[] }> {
  const results = await Promise.all(projects.map(async (project) => ({ project, r: await read(project) })));
  return {
    read: results.flatMap(({ project, r }) => (r.ok ? [{ project, data: r.data }] : [])),
    unread: results.flatMap(({ project, r }) => (r.ok ? [] : [{ name: project.name, reason: failureReason(r.error) }])),
  };
}
