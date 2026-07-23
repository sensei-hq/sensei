import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/dojo2-nav';
import { projects, runs, gates, decisions, chat, me } from '$lib/components/kit/fixtures';

// The personal-zone section loader. The ported chunk-2 sections (projects ·
// runs · approve · decide · chat) render off the kit fixtures (presentational —
// real `/v1` wiring is a later chunk); the rest still render the "coming in the
// rebuild" placeholder. Fixture data is gated behind `hasMembership` so a
// membership-less viewer sees an honest empty screen rather than fabricated
// cross-dōjō work (DJ1). An unknown section (not a real nav destination)
// redirects to the landing so the two-nav never dead-ends.
export const load: PageLoad = async ({ params, parent }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { hasMembership } = await parent();
	return {
		section: params.section,
		title: labelForSection(params.section, 'you'),
		// The ported screens' data (empty for a membership-less viewer).
		projects: hasMembership ? projects : [],
		runs: hasMembership ? runs : [],
		gates: hasMembership ? gates : [],
		decisions: hasMembership ? decisions : [],
		chat: hasMembership ? chat : [],
		me
	};
};
