import type { PageLoad } from './$types';

// The effective-constitution preview. Presentational (Chunk 3) — the sample
// projects + their scope ladders are local (`$lib/preview-data`) and the pure
// resolution runs client-side in a `$state` store, so this load fetches NO
// tenant-scoped `/v1` endpoint. Like the Library, previewing your project's
// governance works solo (DJ1) — there is no join-gate here: a membership-less
// user (org === undefined) still gets the full preview. Live `/v1` wiring to a
// real project's resolved rules (dojo.shared_rules) is deferred (see the plan).
export const load: PageLoad = async () => {
	return {};
};
