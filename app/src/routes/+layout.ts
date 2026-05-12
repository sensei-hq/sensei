import { appState } from '$lib/appstate.svelte.js';

// SPA mode for Tauri — disable SSR and prerendering
export const ssr = false;
export const prerender = false;

/**
 * Root layout load — ensures appState config is loaded before page loaders run.
 *
 * Port is already correct from the build-time default (__SENSEI_DEFAULT_PORT__
 * injected by vite.config.ts). This load just fetches daemon config so child
 * page loaders can read appState.config values.
 */
export async function load() {
  if (!appState.loaded) {
    await appState.load();
  }
  return { port: appState.port };
}
