import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { acceptInvite, ClientApiError } from '$lib/client-data';
import { orgHref, youHref } from '$lib/nav';

// The magic-link invite accept landing (F3b). The invitee arrives here signed in
// (the (app) layout requires a session — a magic-link sign-in precedes this), with
// `?token=…`. We redeem it: the accept endpoint gates on the caller's email
// matching the invite, so a leaked link can't be redeemed by anyone else. On
// success we land in the joined dōjō; a gate rejection (wrong email / expired /
// used / unknown) surfaces its reason — never a fabricated membership.
export const load: PageLoad = async ({ url, fetch, parent }) => {
	const { accessToken } = await parent();
	const token = url.searchParams.get('token') ?? '';
	const back = youHref('dojos');
	if (!token) return { error: 'This invite link is missing its token.', back };

	let tenantId: string;
	try {
		const result = await acceptInvite(token, { fetch, accessToken });
		tenantId = result.tenant_id;
	} catch (e) {
		if (e instanceof ClientApiError) return { error: e.message, back };
		throw e;
	}
	// Success — into the dōjō you just joined (orgBySlug resolves it by tenant id).
	redirect(303, orgHref(tenantId));
};
