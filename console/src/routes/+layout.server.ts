import type { LayoutServerLoad } from './$types';

// Surface the session (set by the kavach handle in hooks.server.ts) to every
// route + the client layout, which hydrates the kavach browser instance.
export const load: LayoutServerLoad = ({ locals }) => {
	return {
		session: locals.session,
		user: locals.session?.user ?? null
	};
};
