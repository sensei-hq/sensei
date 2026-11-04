import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { readKnowledgeSources } from './knowledge-sources-load.js';

// The knowledge-source list, or why it could not be read (#288): one fast
// read, so `load()` is the default (guidelines §2.3). The page state owns what
// follows — the add form, the writes, and their re-reads.
export const load: PageLoad = async () => readKnowledgeSources(senseiApi(appState.port));
