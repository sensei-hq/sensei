/**
 * Shared test helpers for Playwright e2e tests.
 *
 * Tests use SENSEI_INSTANCE=e2e at runtime so the daemon talks to the
 * throw-away `sensei_e2e` DB in `~/.sensei-e2e/` — the user's real
 * `sensei` install is never touched.
 */

import { execSync, spawn, type ChildProcess } from 'child_process';
import { existsSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = parseInt(process.env.SENSEI_PORT ?? '7744', 10);
const BIN = process.env.SENSEID_BIN ?? '../../target/debug/senseid';
const LOCAL_FOLDER = (process.env.LOCAL_FOLDER ?? '~/Developer').replace('~', process.env.HOME ?? '');

let daemonProcess: ChildProcess | null = null;

export { PORT, LOCAL_FOLDER };

/** Base URL for daemon API. */
export const DAEMON_URL = `http://127.0.0.1:${PORT}`;

/** Resolve senseid binary path relative to apps/desktop. */
function resolveBin(): string {
  const resolved = path.resolve(__dirname, '..', BIN);
  if (!existsSync(resolved)) {
    throw new Error(`senseid binary not found at ${resolved}. Run: cargo build --manifest-path crates/senseid/Cargo.toml`);
  }
  return resolved;
}

/** Start the daemon with SENSEI_INSTANCE=e2e. Kills any existing instance first. */
export async function startDaemon(): Promise<void> {
  await stopDaemon();

  const bin = resolveBin();
  daemonProcess = spawn(bin, ['start', '--port', String(PORT)], {
    env: { ...process.env, SENSEI_INSTANCE: 'e2e' },
    stdio: 'ignore',
    detached: true,
  });
  daemonProcess.unref();

  // Wait for health endpoint
  for (let i = 0; i < 30; i++) {
    try {
      const resp = await fetch(`${DAEMON_URL}/health`);
      if (resp.ok) return;
    } catch { /* not ready yet */ }
    await sleep(500);
  }
  throw new Error(`Daemon did not start on port ${PORT} within 15s`);
}

/** Stop the daemon. */
export async function stopDaemon(): Promise<void> {
  try {
    await fetch(`${DAEMON_URL}/stop`, { method: 'POST' });
    await sleep(500);
  } catch { /* not running */ }

  if (daemonProcess) {
    daemonProcess.kill();
    daemonProcess = null;
  }
}

/** Reset all daemon data (repos, projects, config). */
export async function resetDaemon(): Promise<void> {
  await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' });
}

/** Wait for a condition to be true, polling every interval ms. */
export async function waitFor(
  fn: () => Promise<boolean>,
  timeoutMs = 30_000,
  intervalMs = 500,
): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (await fn()) return;
    await sleep(intervalMs);
  }
  throw new Error(`waitFor timed out after ${timeoutMs}ms`);
}

/** Fetch JSON from daemon API. */
export async function daemonGet<T>(path: string): Promise<T> {
  const resp = await fetch(`${DAEMON_URL}${path}`);
  return resp.json() as Promise<T>;
}

/** POST JSON to daemon API. */
export async function daemonPost<T>(path: string, body?: unknown): Promise<T> {
  const resp = await fetch(`${DAEMON_URL}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  return resp.json() as Promise<T>;
}

/** Set the daemon port in the browser's localStorage so the app connects to dev daemon. */
export async function setAppPort(page: import('@playwright/test').Page): Promise<void> {
  await page.addInitScript((port) => {
    localStorage.setItem('sensei:port', String(port));
  }, PORT);
}

/**
 * Navigate to a route using SvelteKit's client-side router.
 *
 * WKWebView (Tauri/WebKit) breaks after a full page reload — the initial
 * SvelteKit mount produces empty DOM. Using SvelteKit's own goto() avoids
 * the reload entirely, so components render correctly.
 *
 * Dev mode (Vite running): imports goto() via node_modules URL.
 * Built Tauri app (no Vite): falls back to anchor click, which SvelteKit's
 * router intercepts for client-side pushState navigation (no reload).
 */
export async function navigateTo(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
  route: string,
): Promise<void> {
  await tauriPage.evaluate(`
    (async function() {
      await new Promise(r => setTimeout(r, 200));
      try {
        var nav = await import('/node_modules/@sveltejs/kit/src/runtime/app/navigation.js');
        await nav.goto(${JSON.stringify(route)});
      } catch {
        // Built Tauri app: anchor click triggers SvelteKit client-side routing
        var a = document.createElement('a');
        a.href = ${JSON.stringify(route)};
        document.body.appendChild(a);
        a.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
        document.body.removeChild(a);
      }
    })()
  `);
  await sleep(800);
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Navigate to a gated observatory screen, RE-navigating until its root selector
 * mounts (or the budget elapses).
 *
 * The observatory shell is gated by `reroute` (health + setup). Two async
 * conditions delay a screen mounting right after a test seeds setup:
 *   1. Cold health-bootstrap — on a fresh e2e DB the daemon's health probe can
 *      take ~30s to reach 'ok'. While it isn't ok, every gated route reroutes
 *      to /health.
 *   2. Setup reconciliation — `wizardState.setupComplete` reconciles from the
 *      daemon config ONLY inside an `$effect` that fires when `healthState.isOk`
 *      *changes* (wizard-state.svelte.ts). A test's PUT of `setup_complete=1`
 *      doesn't itself trigger a transition, so `reroute` only sees it once the
 *      next health transition reconciles — which on a cold boot lands with the
 *      health-ready transition ~50s in. Until then a gated route reroutes to
 *      /setup.
 * In both cases a SINGLE navigation lands on the wrong route and the target
 * selector never appears — waiting longer on it can't help (we're elsewhere).
 * Re-navigating once the conditions settle mounts the screen, so we retry the
 * whole navigation. The budget must comfortably exceed the observed cold-boot
 * health-ready time (~50s on a loaded box). Deterministic: polls for a stable
 * selector, no fixed sleeps.
 */
export async function navigateToScreen(
  tauriPage: {
    evaluate: (script: string) => Promise<unknown>;
    waitForSelector: (selector: string, timeout?: number) => Promise<void>;
  },
  route: string,
  screenSelector: string,
  totalMs = 120_000,
): Promise<void> {
  const deadline = Date.now() + totalMs;
  let lastErr: unknown;
  for (;;) {
    await navigateTo(tauriPage, route);
    try {
      await tauriPage.waitForSelector(screenSelector, 8_000);
      return;
    } catch (e) {
      lastErr = e;
      if (Date.now() >= deadline) break;
    }
  }
  throw lastErr instanceof Error
    ? lastErr
    : new Error(`navigateToScreen: '${route}' never mounted '${screenSelector}' within ${totalMs}ms`);
}

/** Captured runtime-error buffers from the webview. */
export interface ErrBuf {
  /** `console.error(...)` invocations, joined per call. */
  console: string[];
  /** Uncaught `window` `error` events. */
  error: string[];
  /** Uncaught `unhandledrejection` events. */
  rejection: string[];
}

/**
 * Install a runtime-error trap in the webview. Wraps `console.error` and
 * listens for uncaught errors + unhandled rejections, stashing them on a
 * global we read back after an interaction. Install it BEFORE a client-side
 * navigation so a throw during the route's mount is captured — the global
 * survives SvelteKit's reload-free navigation.
 *
 * Buffers reset on each install; the `console.error` wrap happens once so we
 * never stack wrappers across tests. Pair with {@link readErrors}.
 */
export async function installErrorTrap(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      window.__e2e_err__ = { console: [], error: [], rejection: [] };
      if (!window.__e2e_hooked__) {
        window.__e2e_hooked__ = true;
        var orig = console.error.bind(console);
        console.error = function() {
          try { window.__e2e_err__.console.push(Array.prototype.map.call(arguments, String).join(' ')); } catch (e) { /* ignore */ }
          return orig.apply(null, arguments);
        };
        window.addEventListener('error', function(e) { window.__e2e_err__.error.push(String((e && (e.message || e.error)) || e)); });
        window.addEventListener('unhandledrejection', function(e) { window.__e2e_err__.rejection.push(String(e && e.reason)); });
      }
    })()
  `);
}

/** Read the buffers stashed by {@link installErrorTrap}. */
export async function readErrors(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
): Promise<ErrBuf> {
  return (await tauriPage.evaluate(
    `window.__e2e_err__ || { console: [], error: [], rejection: [] }`,
  )) as ErrBuf;
}

/**
 * Poll a DOM predicate evaluated INSIDE the webview until it returns `true`.
 * `expr` must be a JS expression string (e.g. `document.querySelectorAll('x').length === 1`).
 * A deterministic alternative to arbitrary sleeps when waiting on a client-side
 * refetch or re-render to land.
 */
export async function waitForDom(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
  expr: string,
  timeoutMs = 15_000,
): Promise<void> {
  await waitFor(async () => (await tauriPage.evaluate(expr)) === true, timeoutMs);
}
