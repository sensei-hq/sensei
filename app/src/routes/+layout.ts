import { redirect } from '@sveltejs/kit';
import { appState } from '$lib/appstate.svelte.js';
import { gateTarget } from '$lib/boot-gate.js';
import type { LayoutLoad } from './$types.js';

// SPA mode for Tauri — disable SSR and prerendering.
//
// appState.load() is intentionally NOT called here. The /health route does
// not need daemon config, and forcing a load() at the root made every
// navigation issue a /api/config round-trip — including before the daemon
// was even up. Each route group that needs config loads it in its own
// +layout.ts (see `(observatory)/+layout.ts`, `(workspace)/+layout.ts`,
// `(config)/+layout.ts`).
export const ssr = false;
export const prerender = false;

/** The boot gate. Reading `url.pathname` makes SvelteKit re-run this on every
 *  navigation, so the gate is asked afresh each time (see `boot-gate.ts`). */
export const load: LayoutLoad = ({ url }) => {
  const target = gateTarget(url.pathname, { healthOk: appState.healthOk, setupOk: appState.setupOk });
  if (target) redirect(307, target);
};
