import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { readLibraries } from './libraries-load.js';

// The library list, or why it could not be read (#288): one fast read, so
// `load()` is the default (guidelines §2.3). The page state owns what follows —
// the search, the selection and Retry.
export const load: PageLoad = async () => readLibraries(senseiApi(appState.port));
