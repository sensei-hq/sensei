import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { readExtensions } from '../settings-reads';

// The installed extensions, or why they could not be read (#288).
export const load: PageLoad = async () => readExtensions(senseiApi(appState.port));
