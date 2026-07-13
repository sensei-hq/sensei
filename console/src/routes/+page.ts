import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// The console root is the sign-in surface. Send visitors straight to /signin
// (the org picker + guarded console group live behind auth).
export const load: PageLoad = () => {
	redirect(307, '/signin');
};
