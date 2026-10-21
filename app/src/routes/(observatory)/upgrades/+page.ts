import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadUpgrades } from './upgrades-load.js';

export const load: PageLoad = () => loadUpgrades(senseiApi(appState.port));
