import { appState } from '$lib/appstate.svelte.js';
import { senseiApi } from '$lib/api.js';
import type { LayoutLoad } from './$types.js';
import { loadWorkspaceLayout } from './workspace-layout.js';

// (workspace)/+layout.ts has already ensured appState is loaded.
export const load: LayoutLoad = ({ params }) => loadWorkspaceLayout(senseiApi(appState.port), params.id);
