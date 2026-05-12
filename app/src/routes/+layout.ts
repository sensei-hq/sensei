import { getDaemonPort } from '$lib/bootstrap.js';
import { appState } from '$lib/appstate.svelte.js';

// SPA mode for Tauri — disable SSR and prerendering
export const ssr = false;
export const prerender = false;

/**
 * Root layout load — resolves daemon port before any child page load runs.
 *
 * This guarantees appState.port is correct (7744 prod / 7745 dev) before
 * any page loader calls senseiApi(appState.port). Without this, page loaders
 * race against the port resolution and hit the wrong port.
 */
export async function load() {
  if (!appState.loaded) {
    try {
      const port = await getDaemonPort();
      await appState.setPort(port);
    } catch { /* keep default 7744 */ }
    await appState.load();
  }
  return { port: appState.port };
}
