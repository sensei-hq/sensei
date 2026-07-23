import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/dojo2-nav';
import {
	projects,
	runs,
	gates,
	decisions,
	chat,
	me,
	stance,
	ladder,
	rulePacks,
	dojos,
	contributionsMine,
	contributionsDownstream,
	contributionsStat
} from '$lib/components/kit/fixtures';

// The personal-zone section loader. The ported sections (projects · runs ·
// approve · decide · chat · rules · packs · dojos · contributions) render off
// the kit fixtures (presentational — real `/v1` wiring is a later chunk); any
// remaining section still renders the "coming in the rebuild" placeholder.
// Membership-scoped data (cross-dōjō work + memberships) is gated behind
// `hasMembership` so a membership-less viewer sees an honest empty screen
// rather than fabricated work (DJ1). Your own standing governance — the stance
// dials, personal constitution, and adoptable rule packs — is NOT gated: it is
// yours whether or not you belong to a dōjō. An unknown section (not a real nav
// destination) redirects to the landing so the two-nav never dead-ends.
export const load: PageLoad = async ({ params, parent }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { hasMembership } = await parent();
	return {
		section: params.section,
		title: labelForSection(params.section, 'you'),
		// The ported screens' data (membership-scoped data empty for a
		// membership-less viewer).
		projects: hasMembership ? projects : [],
		runs: hasMembership ? runs : [],
		gates: hasMembership ? gates : [],
		decisions: hasMembership ? decisions : [],
		chat: hasMembership ? chat : [],
		me,
		// Your own governance — always yours, membership or not.
		stance,
		ladder,
		rulePacks,
		// Memberships + upstream/downstream sharing — membership-scoped.
		dojos: hasMembership ? dojos : [],
		contributionsMine: hasMembership ? contributionsMine : [],
		contributionsDownstream: hasMembership ? contributionsDownstream : [],
		contributionsStat: hasMembership
			? contributionsStat
			: { approved: 0, pending: 0, helped: 0 }
	};
};
