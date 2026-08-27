import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// Reaching /signin while already authenticated means the sign-in has ALREADY
// succeeded — forward, do not re-present the form.
//
// This is what completes an OAuth return. The first server render of the return
// sees no session (the token arrives client-side, and the cookie is set after
// that), so the visitor is bounced here. kavach then syncs the session and calls
// `invalidateAll()`, which re-runs this load — now with a session — and lands
// them home. Without it the user sits on the login page holding a valid session,
// which is exactly what happened on 2026-08-27.
export const load: PageLoad = async ({ parent }) => {
	const { session } = await parent();
	if (session) redirect(307, '/you');
};
