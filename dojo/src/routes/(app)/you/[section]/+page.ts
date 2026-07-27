import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/nav';
import { toKitDojos } from '$lib/chrome';
import { stance, ladder, rulePacks } from '$lib/components/kit/fixtures';

// The personal-zone section loader — the non-inbox destinations (projects ·
// rules · packs · dojos · contributions). `dojos` binds REAL memberships (from
// the layout). Projects + Contributions have NO backing route yet, so rather
// than show fabricated repos / a fake "helped 612" to a real user (F4 honesty),
// they render their honest empty state until a /v1 route lands. Your own
// governance (stance · constitution · rule packs) stays fixture-backed pending
// its own route. An unknown/retired section (incl. the old approve/decide/chat/
// runs, no longer in YOU_SECTIONS) redirects to the Inbox landing.
export const load: PageLoad = async ({ params, parent }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { memberships } = await parent();
	const section = params.section;
	return {
		section,
		title: labelForSection(section, 'you'),
		// Your own governance — always yours (still fixture-backed pending its route).
		stance,
		ladder,
		rulePacks,
		// Memberships — real, from the layout's `listUserOrgs`, mapped to KitDojo.
		dojos: toKitDojos(memberships),
		// No backing route yet → honest empty, never fabricated (F4).
		projects: [],
		contributionsMine: [],
		contributionsDownstream: [],
		contributionsStat: { approved: 0, pending: 0, helped: 0 }
	};
};
