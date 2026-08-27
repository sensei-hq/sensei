import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// The console root is also the OAuth RETURN TARGET: Supabase redirects to
// `site_url`, which is the bare origin, so every provider sign-in lands here
// before anything else runs.
//
// This redirected to /signin UNCONDITIONALLY, which made a successful sign-in
// indistinguishable from a failed one — kavach set the session cookie correctly
// (POST /auth/session → 200) and the user was still looking at the login form
// (observed 2026-08-27). Send an authenticated visitor to their personal zone;
// only an anonymous one needs the form.
export const load: PageLoad = async ({ parent }) => {
	const { session } = await parent();
	redirect(307, session ? '/you' : '/signin');
};
