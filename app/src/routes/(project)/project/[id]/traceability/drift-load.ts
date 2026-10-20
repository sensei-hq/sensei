import { readOutcome, type ApiResult } from '$lib/api.js';
import type { DriftItem } from '$lib/types.js';

type Drift = { items: DriftItem[]; total: number; drifted: number; broken: number; indexedDocs?: number };

/** One project's doc drift and its counts, or why it could not be read — not
 *  the "0 tracked" an empty default made of a failure (#288). */
export async function loadProjectDrift(api: { tryGetProjectDrift: (id: string) => Promise<ApiResult<Drift>> }, id: string) {
  const { data, error } = readOutcome(await api.tryGetProjectDrift(id));
  return {
    driftItems: data?.items ?? [],
    total: data?.total ?? 0,
    drifted: data?.drifted ?? 0,
    broken: data?.broken ?? 0,
    // How many documents the scan had to check (#290); null when the daemon did
    // not say — unknown, never read as "none".
    indexedDocs: data?.indexedDocs ?? null,
    error,
  };
}
