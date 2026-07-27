import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/nav';
import { toKitDojos } from '$lib/chrome';
import {
	projects,
	stance,
	ladder,
	rulePacks,
	contributionsMine,
	contributionsDownstream,
	contributionsStat
} from '$lib/components/kit/fixtures';

// The personal-zone section loader — the non-inbox destinations (projects ·
// rules · packs · dojos · contributions). The relay surfaces (approve · decide ·
// chat · runs) folded into the Inbox landing, so this no longer fetches
// runs/gates. `dojos` binds REAL memberships (from the layout); the rest are
// presentational off the kit fixtures pending their own /v1 routes (F4). An
// unknown or retired section (incl. the old approve/decide/chat/runs, which
// YOU_SECTIONS no longer lists) redirects to the Inbox landing.
export const load: PageLoad = async ({ params, parent }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { hasMembership, memberships } = await parent();
	const section = params.section;
	return {
		section,
		title: labelForSection(section, 'you'),
		// Your own governance — always yours, membership or not (still fixture-backed).
		stance,
		ladder,
		rulePacks,
		// Memberships — real, from the layout's `listUserOrgs`, mapped to KitDojo.
		dojos: toKitDojos(memberships),
		// Still presentational pending their routes (F4).
		projects: hasMembership ? projects : [],
		contributionsMine: hasMembership ? contributionsMine : [],
		contributionsDownstream: hasMembership ? contributionsDownstream : [],
		contributionsStat: hasMembership ? contributionsStat : { approved: 0, pending: 0, helped: 0 }
	};
};
