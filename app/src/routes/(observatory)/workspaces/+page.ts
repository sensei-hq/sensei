import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadWorkspacesIndex } from './workspaces-load.js';

export const load: PageLoad = () => loadWorkspacesIndex(senseiApi(appState.port));
