/**
 * Client-side routing guard — two tiers:
 *
 * 1. Health gate (sessionStorage — cleared by WKWebView on cold start)
 *    Bootstrap runs on every app launch. Once all gates pass, the health page
 *    writes `sensei:health=ready` to sessionStorage. Until then, every
 *    navigation is redirected to /health so daemon + postgres are guaranteed
 *    to be up before the app is usable.
 *
 *    sessionStorage is safe here because WKWebView destroys it when the app
 *    process exits. Within a session (app minimised/restored) it persists,
 *    so the user isn't bounced back to the health screen mid-session.
 *
 * 2. Setup gate (localStorage — survives cold starts)
 *    Once the setup wizard finishes, `sensei:setup-complete=1` is written to
 *    localStorage. The guard stops redirecting to /setup/welcome.
 *
 * Imports are restricted to pure modules (no Svelte runes) so this file
 * loads safely before the Svelte runtime initialises. Importing a
 * `.svelte.ts` / `$state`-rune module at hook time causes a blank screen
 * in tauri:dev. `health-cache.ts` is rune-free.
 *
 * Routes always reachable regardless of gate state:
 *   /health  /logs
 *
 * Routes exempt from the setup gate only:
 *   /setup/*
 */

import { initHealthCache, isHealthReady } from '$lib/health-cache.js';

const HEALTH_EXEMPT = new Set(['/health', '/logs', '/upgrade']);
const SETUP_PREFIX = '/setup';

// Cold-start init — owned by health-cache. Either clears stale state (so a
// 'ready' value that survived hot-reload doesn't bypass HealthState) or
// pre-populates 'ready' in bypass mode so reroute passes without a /health
// hop. This is the only sessionStorage interaction here; the actual cache
// writes during the app's lifetime are owned by HealthState.
initHealthCache();

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;

  // ── Tier 0: Upgrade gate ──────────────────────────────────────────────────
  // If the app was just updated, run upgrade steps before health checks.
  // The updater writes `sensei:app-version` to localStorage before restarting;
  // the /upgrade page clears it once the health resolvers + db deploy have completed.
  const pendingUpgrade =
    typeof localStorage !== 'undefined' &&
    localStorage.getItem('sensei:app-version') !== null;

  if (pendingUpgrade && !HEALTH_EXEMPT.has(path) && path !== '/') {
    return '/upgrade';
  }

  // ── Tier 1: Health gate ───────────────────────────────────────────────────
  if (!isHealthReady()) {
    if (HEALTH_EXEMPT.has(path)) return undefined;
    return '/health';
  }

  // ── Tier 2: Setup gate ────────────────────────────────────────────────────
  const setupComplete =
    typeof localStorage !== 'undefined' &&
    localStorage.getItem('sensei:setup-complete') === '1';

  if (!setupComplete) {
    if (HEALTH_EXEMPT.has(path) || path.startsWith(SETUP_PREFIX)) return undefined;
    return '/setup/welcome';
  }

  // Both gates passed — '/' is the observatory (lives in (observatory)/+page.svelte)
  return undefined;
}
