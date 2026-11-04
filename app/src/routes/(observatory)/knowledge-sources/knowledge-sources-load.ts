// The Knowledge sources list read, shared by the route's loader (+page.ts) and
// the screen's state (a write re-reads through it, so a remove or sync touches
// only /api/knowledge/sources). A list that could not be read is the error it
// was, never an empty list the screen would render as "No knowledge sources."
// (#288).
import { failureReason, type ApiResult } from '$lib/api.js';
import type { SenseiApi } from '$lib/api.js';
import type { KnowledgeSource } from '$lib/types.js';

/** The read the loader and the state share — injected so a spec can fail it. */
export type KnowledgeSourcesListApi = Pick<SenseiApi, 'tryListKnowledgeSources'>;

/** What the screen shows for the list: the sources, or why they could not be read. */
export interface KnowledgeSourcesRead {
  sources: KnowledgeSource[];
  error: string | null;
}

export async function readKnowledgeSources(
  api: KnowledgeSourcesListApi,
): Promise<KnowledgeSourcesRead> {
  const read: ApiResult<{ sources: KnowledgeSource[] }> = await api.tryListKnowledgeSources();
  return read.ok
    ? { sources: read.data.sources, error: null }
    : { sources: [], error: failureReason(read.error) };
}
