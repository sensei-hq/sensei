import { readOutcome, type ApiResult } from '$lib/api.js';
import type { MemoryShareBatch, ProjectMemory } from '$lib/types.js';

/** The reads the project Memories page makes — injected so a spec can fail them. */
export interface MemoriesApi {
  tryGetProjectMemories: (id: string) => Promise<ApiResult<{ active: ProjectMemory[]; total: number }>>;
  tryListMemoryShareBatches: (id: string, status?: string) => Promise<ApiResult<{ batches: MemoryShareBatch[] }>>;
}

/**
 * The project's active memories and its PROPOSED share batches (approved and
 * rejected verdicts stay in the audit fold). Either read failing is the page's
 * error (#288) — not "Nothing to remember yet."
 */
export async function loadProjectMemories(api: MemoriesApi, id: string) {
  const [mem, batches] = await Promise.all([api.tryGetProjectMemories(id), api.tryListMemoryShareBatches(id, 'proposed')]);
  const [m, b] = [readOutcome(mem), readOutcome(batches)];
  return {
    memories: m.data?.active ?? [],
    total: m.data?.total ?? 0,
    proposedBatches: b.data?.batches ?? [],
    error: m.error ?? b.error,
  };
}
