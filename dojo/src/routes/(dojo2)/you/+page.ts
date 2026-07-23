import type { PageLoad } from './$types';
import { needsYou, runs, projects } from '$lib/components/kit/fixtures';

// The Your-work landing data. Presentational this chunk: the band / live-runs /
// active-projects render off the ported kit fixtures (the real `/v1` wiring is a
// later chunk). Kept in `+page.ts` (universal) so the same shapes flow under
// SSR and in unit tests. The landing degrades gracefully for a membership-less
// viewer — the shell's `hasMembership` gates the fixtures so a solo user sees an
// honest empty landing rather than fabricated cross-dōjō work (DJ1).
export const load: PageLoad = async ({ parent }) => {
	const { hasMembership } = await parent();
	return {
		needsYou: hasMembership ? needsYou : [],
		runs: hasMembership ? runs : [],
		projects: hasMembership ? projects : []
	};
};
