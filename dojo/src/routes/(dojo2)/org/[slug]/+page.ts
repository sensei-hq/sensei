import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgBySlug } from '$lib/dojo2-chrome';

// The org home resolves its org from the URL slug against the caller's real
// memberships (not just the tenant cookie), so a direct link to /org/{slug}
// shows the right dōjō. A slug the caller isn't a member of redirects to the
// personal landing — never a fabricated org (DJ1).
export const load: PageLoad = async ({ parent, params }) => {
	const { memberships } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	return { slug: params.slug, orgName: org.name };
};
