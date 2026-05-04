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

const HEALTH_EXEMPT = new Set(['/health', '/logs']);
const SETUP_PREFIX = '/setup';

export function reroute({ url }: { url: URL }): string | undefined {
  const path = url.pathname;

  // ── Tier 1: Health gate ───────────────────────────────────────────────────
  const healthReady =
    typeof sessionStorage !== 'undefined' &&
    sessionStorage.getItem('sensei:health') === 'ready';

  if (!healthReady) {
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

  // Both gates passed — route root to the main view, allow all else
  if (path === '/') return '/observatory';

  return undefined;
}
