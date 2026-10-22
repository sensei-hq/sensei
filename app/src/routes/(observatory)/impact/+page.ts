import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadImpactAcross } from '$lib/impact-load.js';

// Every workspace's measured verdicts — a rec with a reasoning trace or a
// non-pending verdict, so a fresh install is honestly empty (#288).
export const load: PageLoad = () => loadImpactAcross(senseiApi(appState.port));
