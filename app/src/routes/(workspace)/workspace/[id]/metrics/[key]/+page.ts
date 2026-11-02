import type { PageLoad } from './$types.js';
import type { Grain } from '../metrics-state.svelte.js';

// Master-detail drill-down for one signal. Reads `?grain=daily|weekly|monthly`
// so the grain toggle is a plain link. The reads themselves — the values and
// registry the rail needs, this metric's series at the grain, its daily series
// for the datapoint drill-down, the pending recommendations — live in the
// workspace's metrics state, shared with the landing and kept across metrics
// and grains (load() re-read all of them on every change).
const GRAINS: readonly Grain[] = ['daily', 'weekly', 'monthly'];

export const load: PageLoad = ({ params, url }) => {
    const requested = url.searchParams.get('grain') ?? 'daily';
    const grain: Grain = (GRAINS as readonly string[]).includes(requested) ? (requested as Grain) : 'daily';
    return { selectedKey: params.key, grain };
};
