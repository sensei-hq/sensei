/**
 * Client-side routing guard — two tiers:
 *
 * 1. Health gate: if bootstrap has never completed, redirect to /health.
 *    Once all gates pass, the health page sets `sensei:health = ready` in
 *    localStorage and this guard stops intercepting health-unrelated routes.
 *
 * 2. Setup gate: if health is ready but setup hasn't been finished, redirect
 *    to /setup/welcome. Once the wizard completes, `sensei:setup-complete = 1`
 *    is written and the guard allows through.
 *
 * Routes that are always reachable regardless of guard state:
 *   /health  /logs
 *
 * Routes exempt from the setup gate (allowed while setup is in progress):
 *   /setup/*
 */

const HEALTH_EXEMPT = new Set(['/health', '/logs']);
const SETUP_PREFIX = '/setup';

export function reroute({ url }: { url: URL }): string | undefined {
  if (typeof localStorage === 'undefined') return undefined;

  const path = url.pathname;

  // ── Tier 1: Health gate ───────────────────────────────────────────────────
  const healthReady = localStorage.getItem('sensei:health') === 'ready';

  if (!healthReady) {
    // Already heading to an exempt route — let it through
    if (HEALTH_EXEMPT.has(path)) return undefined;
    return '/health';
  }

  // ── Tier 2: Setup gate ────────────────────────────────────────────────────
  const setupComplete = localStorage.getItem('sensei:setup-complete') === '1';

  if (!setupComplete) {
    // Allow /health, /logs, and any /setup/* page
    if (HEALTH_EXEMPT.has(path) || path.startsWith(SETUP_PREFIX)) return undefined;
    return '/setup/welcome';
  }

  // Both gates passed — route root to the main view
  if (path === '/') return '/observatory';

  return undefined;
}
