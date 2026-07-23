import type { LayoutServerLoad } from './$types';
import { loadConsoleContext } from '$lib/server/console-load';

// The tenant + membership + auth surface for every guarded console screen.
// Delegates to the shared `loadConsoleContext` (also used by the `(dojo2)`
// shell) so the two shells resolve the identical context from ONE place and can
// never drift. See `$lib/server/console-load` for the field contract (DJ1).
//
// A server load (not a universal `+layout.ts`) because only the server can read
// `locals` and the httpOnly-free-but-server-owned cookie.
export const load: LayoutServerLoad = ({ cookies, url, locals }) =>
	loadConsoleContext({ cookies, url, locals });
