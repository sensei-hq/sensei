import type { PageLoad } from './$types';

// The constitution library. Presentational (Chunk 2) — the pack catalog is
// local (`$lib/library-data`) and the working selection lives in a client
// `$state` store, so this load does NOT fetch any tenant-scoped `/v1` endpoint.
// It only reads the resolved chrome org from the layout to name the target
// constitution in the footer ("Add N to <scope>"). A membership-less user (DJ1,
// `org === undefined`) still gets the full library — seeding your own rules
// works solo, so there is no join-gate here — with a neutral scope label.
// Live `/v1` wiring to real `dojo.shared_rules` is deferred (see the plan).
export const load: PageLoad = async ({ parent }) => {
	const { org } = await parent();
	return { scopeName: org?.name ?? 'your constitution' };
};
