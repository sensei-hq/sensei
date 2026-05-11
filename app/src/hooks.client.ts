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
 * No module imports — only Web APIs — so this runs safely before the Svelte
 * runtime initialises (importing $state-rune modules at hook time causes
 * a blank screen in tauri:dev).
 *
 * Routes always reachable regardless of gate state:
 *   /health  /logs
 *
 * Routes exempt from the setup gate only:
 *   /setup/*
 */

const HEALTH_EXEMPT = new Set(['/health', '/logs', '/upgrade']);
const SETUP_PREFIX = '/setup';

// Clear the health flag on every cold start so a stale sessionStorage value
// from a previous dev session never bypasses bootstrap. WKWebView does clear
// sessionStorage when the app process exits, but NOT across hot-reloads or
// when the renderer process stays alive (common in tauri:dev).
// This module runs once at startup before reroute, so clearing here is safe.
if (typeof sessionStorage !== 'undefined') {
  sessionStorage.removeItem('sensei:health');
}

// VITE_BYPASS_HEALTH=true is set in tauri:vite-dev. In that mode the Tauri
// sidecar is not embedded, so the health check cannot run. Health is treated
// as already passed so the app is directly usable for frontend development.
// This value is replaced at build/serve time by Vite — it is never 'true'
// in a production or tauri:dev (binary) build.
const BYPASS_HEALTH = import.meta.env.VITE_BYPASS_HEALTH === 'true';

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;

  // ── Tier 0: Upgrade gate ──────────────────────────────────────────────────
  // If the app was just updated, run upgrade steps before health checks.
  // The updater writes `sensei:app-version` to localStorage before restarting;
  // the /upgrade page clears it once brew bundle + db deploy have completed.
  const pendingUpgrade =
    !BYPASS_HEALTH &&
    typeof localStorage !== 'undefined' &&
    localStorage.getItem('sensei:app-version') !== null;

  if (pendingUpgrade && !HEALTH_EXEMPT.has(path) && path !== '/') {
    return '/upgrade';
  }

  // ── Tier 1: Health gate ───────────────────────────────────────────────────
  const healthReady =
    BYPASS_HEALTH ||
    (typeof sessionStorage !== 'undefined' &&
      sessionStorage.getItem('sensei:health') === 'ready');

  if (!healthReady) {
    // '/' is handled by root +page.svelte onMount — letting it redirect
    // via onMount instead of reroute avoids the WKWebView TDZ bug that
    // occurs when too many modules load simultaneously during initial mount.
    if (HEALTH_EXEMPT.has(path) || path === '/') return undefined;
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

  // Both gates passed — route root to the main view, allow all else
  if (path === '/') return '/observatory';

  return undefined;
}
