/**
 * Client-side routing guard — two tiers:
 *
 * 1. Health gate (in-memory, per session)
 *    Bootstrap runs on every cold start. appState.healthReady starts as false
 *    and is set to true once all gates pass in the health page. Until then,
 *    every navigation is redirected to /health so daemon + postgres are
 *    guaranteed to be up before the app is usable.
 *
 * 2. Setup gate (localStorage, permanent)
 *    Once the setup wizard finishes, `sensei:setup-complete=1` is written to
 *    localStorage. The guard skips /setup/welcome on subsequent cold starts.
 *
 * Routes always reachable regardless of gate state:
 *   /health  /logs
 *
 * Routes exempt from the setup gate only:
 *   /setup/*
 */

import { appState } from '$lib/appstate.svelte.js';

const HEALTH_EXEMPT = new Set(['/health', '/logs']);
const SETUP_PREFIX = '/setup';

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;

  // ── Tier 1: Health gate (in-memory — resets on every cold start) ──────────
  if (!appState.healthReady) {
    if (HEALTH_EXEMPT.has(path)) return undefined;
    return '/health';
  }

  // ── Tier 2: Setup gate (localStorage — survives cold starts) ──────────────
  const setupComplete =
    typeof localStorage !== 'undefined' &&
    localStorage.getItem('sensei:setup-complete') === '1';

  if (!setupComplete) {
    if (HEALTH_EXEMPT.has(path) || path.startsWith(SETUP_PREFIX)) return undefined;
    return '/setup/welcome';
  }

  // Both gates passed — route root to the main view, allow all else
  if (path === '/') return '/observatory';

  return undefined;
}
