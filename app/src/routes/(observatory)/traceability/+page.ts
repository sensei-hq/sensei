import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadDriftAcross } from './traceability-load.js';

export const load: PageLoad = () => loadDriftAcross(senseiApi(appState.port));
