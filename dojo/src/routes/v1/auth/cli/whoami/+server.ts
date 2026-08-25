// GET /v1/auth/cli/whoami — prove an access token is actually usable.
//
// A refresh succeeding shows the REFRESH token is live; it says nothing about
// whether the access token it produced is accepted. Without this, senseid would
// report "signed in" on the strength of a refresh and only discover otherwise
// when a sync failed — the exact lie the status endpoint exists to prevent.
//
// Unlike the rest of this group it authenticates, via the same JWT plane every
// other /v1 route uses: being rejected here IS the answer.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, email } = await resolveCaller(request, locals);
		return Response.json({ userId, email });
	} catch (e) {
		if (e instanceof Response) return e;
		return apiError(500, 'could not verify the token');
	}
};
