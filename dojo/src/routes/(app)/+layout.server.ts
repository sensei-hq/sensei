import type { LayoutServerLoad } from './$types';
import { loadConsoleContext } from '$lib/server/console-load';

// The tenant + membership + auth surface for the new dojo shell (`/you` +
// `/org/[slug]`). Delegates to the shared `loadConsoleContext` — the SAME
// resolver the shipped `(console)` group uses — so the two shells can never
// drift on how membership, the selected tenant, or the chrome org resolve (DJ1).
export const load: LayoutServerLoad = ({ cookies, url, locals }) =>
	loadConsoleContext({ cookies, url, locals });
