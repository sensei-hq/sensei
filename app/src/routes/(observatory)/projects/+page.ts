import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadProjectsIndex } from './projects-load.js';

export const load: PageLoad = () => loadProjectsIndex(senseiApi(appState.port));
