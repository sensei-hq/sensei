import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { readAssistants } from '../settings-reads';

// The detected assistants, or why they could not be detected (#288).
export const load: PageLoad = async () => readAssistants(senseiApi(appState.port));
