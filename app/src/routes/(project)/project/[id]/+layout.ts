import { appState } from '$lib/appstate.svelte.js';
import { senseiApi } from '$lib/api.js';
import type { LayoutLoad } from './$types.js';
import { loadProjectLayout } from './project-layout.js';

// (project)/+layout.ts has already ensured appState is loaded.
export const load: LayoutLoad = ({ params }) => loadProjectLayout(senseiApi(appState.port), params.id);
