import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { guessUserName } from '$lib/setup/user-name';
import { readPreferences } from '../settings-reads';

// The saved preferences, or why they could not be read (#288). An empty display
// name is filled from the system user name, as setup does.
export const load: PageLoad = async () => readPreferences(senseiApi(appState.port), guessUserName);
