import { expect, type TauriPageLike } from './fixtures';
/**
 * Shared test helpers for Playwright e2e tests.
 *
 * Tests use SENSEI_INSTANCE=e2e at runtime so the daemon talks to the
 * throw-away `sensei_e2e` DB in `~/.sensei-e2e/` — the user's real
 * `sensei` install is never touched.
 */

import { execSync, spawn, type ChildProcess } from 'child_process';
import { existsSync, readFileSync, rmSync } from 'fs';
import { isUniformPng, PAGE_BAND } from './lib/png-blank';
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

/** Reset all daemon data (repos, workspaces, config). */
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

/**
 * Remove every watch root under `prefix`, and what it indexed. A spec that adds
 * roots takes them out again: the e2e database persists between runs, and a
 * left-over root fails the next spec that expects none.
 */
export async function removeRootsUnder(prefix: string): Promise<void> {
  const roots = await daemonGet<Array<{ id: string; path: string }>>('/api/scan/roots');
  for (const r of roots.filter((r) => r.path.startsWith(prefix))) {
    const resp = await fetch(`${DAEMON_URL}/api/scan/roots/${r.id}?repositories=remove`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`removing root ${r.path}: HTTP ${resp.status}`);
  }
}

/**
 * A workspace for a spec to scope to, SEEDED rather than assumed: `reset-e2e-db`
 * drops `sensei_e2e` before a run, so a fresh database has no workspaces and a spec
 * reading `/api/workspaces[0]` throws before it tests anything. Reused by name, so
 * a spec's workspace is the same across runs and never one another spec left.
 * Nothing is indexed under it — screens land in their empty terminal state.
 */
export async function seedWorkspace(name: string, description: string): Promise<string> {
  const existing = await daemonGet<Array<{ id: string; name: string }>>('/api/workspaces');
  const mine = existing.find((p) => p.name === name);
  if (mine) return mine.id;
  const created = await daemonPost<{ ok: boolean; id: string }>('/api/workspaces', { name, description });
  return created.id;
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
 * The observatory shell is gated by the boot gate (routes/+layout.ts) (health + setup). Two async
 * conditions delay a screen mounting right after a test seeds setup:
 *   1. Cold health-bootstrap — on a fresh e2e DB the daemon's health probe can
 *      take ~30s to reach 'ok'. While it isn't ok, every gated route redirects
 *      to /health.
 *   2. Setup reconciliation — `wizardState.setupComplete` reconciles from the
 *      daemon config ONLY inside an `$effect` that fires when `healthState.isOk`
 *      *changes* (wizard-state.svelte.ts). A test's PUT of `setup_complete=1`
 *      doesn't itself trigger a transition, so the gate only sees it once the
 *      next health transition reconciles — which on a cold boot lands with the
 *      health-ready transition ~50s in. Until then a gated route redirects to
 *      /setup.
 * In both cases a SINGLE navigation lands on the wrong route and the target
 * selector never appears — waiting longer on it can't help (we're elsewhere).
 * Re-navigating once the conditions settle mounts the screen, so we retry the
 * whole navigation — from the page the gate settled on, or after 30s with nothing
 * moved; never while a slow load is still in flight. The budget must comfortably
 * exceed the observed cold-boot health-ready time (~50s on a loaded box).
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
  const target = route.split(/[?#]/)[0];
  const sel = JSON.stringify(screenSelector);
  // Arrival is the path AND the selector: the selector alone can match on the page a
  // redirect landed on (the boot gate sends an early navigation through /health to Today,
  // and the rail is on both). The path moves only once the route's loaders finish, so a
  // slow load is waited out, not re-issued: re-navigating restarts the load, and under a
  // busy daemon a load slower than the retry interval never landed at all.
  let origin = (await tauriPage.evaluate('location.pathname')) as string;
  await navigateTo(tauriPage, route);
  let issued = Date.now();
  let at = origin;
  while (Date.now() < deadline) {
    const probe = (await tauriPage
      .evaluate(`JSON.stringify({ at: location.pathname, ready: !!document.querySelector(${sel}) })`)
      .catch(() => null)) as string | null;
    if (probe) {
      const now = JSON.parse(probe) as { at: string; ready: boolean };
      at = now.at;
      if (at === target && now.ready) return;
      const redirected = at !== target && at !== origin;
      const stalled = at !== target && Date.now() - issued > 30_000;
      if (redirected || stalled) {
        // Somewhere else (a gate), or nothing moved: once settled, ask again from there.
        await new Promise((r) => setTimeout(r, 1_000));
        origin = at;
        await navigateTo(tauriPage, route);
        issued = Date.now();
        continue;
      }
    }
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error(
    at === target
      ? `navigateToScreen: '${route}' never mounted '${screenSelector}' within ${totalMs}ms`
      : `navigateToScreen: asked for '${route}', the window is on '${at}'`,
  );
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

/**
 * Mark setup complete — in the daemon's config AND in the page's cached state.
 *
 * the boot gate (routes/+layout.ts) redirects every page to `/setup/welcome` until setup is complete,
 * and `make reset-e2e-db` drops the row that says it is. A spec that skips this
 * passes only when an earlier run happened to leave `setup_complete` behind:
 * the whole Settings rail failed on a reset DB for exactly that reason.
 *
 * Was copied verbatim into eight specs; this is the one copy.
 */
export async function seedSetupComplete(tauriPage: TauriPageLike): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  // The gate reads the wizard's in-memory flag, which reconciles against the
  // daemon only when the setup layout mounts (or on a health TRANSITION) — so a
  // config write alone changes nothing in a running page, and writing
  // localStorage behind the wizard's back is ignored until a boot. A spec that
  // followed one resetting setup sat on /setup/welcome for its whole budget.
  //
  // Do what the app does instead: mount the setup layout. Its load() reads
  // setup_complete=1 and sends a completed setup on to `/`. (A full reload would
  // also re-read it, but WKWebView mounts nothing after one — see navigateTo.)
  await navigateTo(tauriPage, '/logs');
  await navigateTo(tauriPage, '/setup/welcome');
  await waitFor(async () => {
    try {
      return (await tauriPage.evaluate(
        `!!document.querySelector('[data-component="observatory-sidebar"]')`,
      )) === true;
    } catch {
      return false;
    }
  }, 30_000);
}

// ── Setup wizard navigation (shared by every wizard spec) ─────────────────────

// `?force=1` is REQUIRED in e2e: the (config) layout's onMount redirects to the
// observatory when `appState.setupOk` (setup already complete) unless the URL
// carries `force=1` — the "re-run setup" escape hatch. globalSetup seeds
// setup-complete so the observatory specs work, so without force the wizard
// bounces to / and the rail never mounts.
export const WELCOME = '/setup/welcome?force=1';

export async function startAtWelcome(tauriPage: TauriPageLike): Promise<void> {
  // /logs is ALWAYS_REACHABLE — visiting it unmounts the (config) layout so the
  // next navigation remounts fresh (onMount → loadWizardData with the reset
  // daemon).
  await navigateTo(tauriPage, '/logs');
  // /setup/* is gated behind healthOk (the boot gate (routes/+layout.ts)); on a cold e2e DB the
  // health probe takes ~50s to reach 'ok', so RE-navigate until the rail mounts.
  // Mirrors the observatory shell's navigateToScreen.
  await navigateToScreen(tauriPage, WELCOME, '[data-testid="rail"]');
}

export async function clickAndExpectNav(
  tauriPage: TauriPageLike,
  selector: string,
  expectedPath: string,
  timeout = 10_000,
): Promise<void> {
  const seen: string[] = [];
  const deadline = Date.now() + timeout;

  await tauriPage.click(selector);

  let reached = false;
  while (Date.now() < deadline) {
    try {
      const p = await tauriPage.evaluate(`window.location.pathname`);
      if (typeof p === 'string') {
        seen.push(p);
        if (p === expectedPath) { reached = true; break; }
      }
    } catch { /* mid-transition */ }
    await new Promise<void>(r => setTimeout(r, 80));
  }

  const unexpected = seen.filter(p => p === '/health');
  expect(unexpected, `Unexpected redirect to /health while navigating to ${expectedPath}`).toHaveLength(0);

  if (!reached) {
    const current = await tauriPage.evaluate(`window.location.pathname`).catch(() => '(unknown)');
    throw new Error(`Timed out (${timeout}ms) waiting for ${expectedPath}. Current: ${current}`);
  }
}

/**
 * Capture the app window to `path`, and say so when the capture is not a
 * picture of the app (#284). Two ways it fails:
 *
 * - `blank`: one flat colour. The window capture needs macOS Screen Recording
 *   permission for Sensei.app, and a rebuilt, re-signed bundle loses it (the
 *   e2e build is signed with a stable identity by `make app-e2e-build`; with
 *   the `sensei-dev` certificate the grant then survives rebuilds).
 * - `unpainted`: the frame and docked devtools paint, the page area does not.
 *
 * Either way the file is removed — a blank PNG left behind reads as evidence —
 * and the cause is logged. The test does not fail on it: its DOM assertions
 * are the evidence; the picture is for a person.
 */
export async function shot(
  tauriPage: { screenshot: (o: { path: string }) => Promise<unknown> },
  path: string,
): Promise<'ok' | 'blank' | 'unpainted'> {
  await tauriPage.screenshot({ path });
  const png = readFileSync(path);
  const result = isUniformPng(png) ? 'blank' : isUniformPng(png, PAGE_BAND) ? 'unpainted' : 'ok';
  if (result !== 'ok') {
    rmSync(path);
    console.warn(
      result === 'blank'
        ? `[shot] ${path}: the capture is blank — grant Sensei.app Screen Recording in System Settings › Privacy & Security (#284)`
        : `[shot] ${path}: the page area did not paint in the capture (#284)`,
    );
  }
  return result;
}

/**
 * Record what the browser console would show as an error (user, 2026-10-27:
 * keep track of console errors): `console.error`, uncaught errors, unhandled
 * rejections, a fetch that fails or answers non-2xx, and a resource (an `<img>`,
 * a script) that fails to load. Installed once per window — SvelteKit routes on
 * the client, so the window and its hooks survive navigation. Idempotent.
 */
export async function watchConsole(tauriPage: { evaluate: (script: string) => Promise<unknown> }): Promise<void> {
  await tauriPage.evaluate(`(function () {
    if (window.__consoleWatch) return 1;
    window.__consoleWatch = true;
    window.__consoleErrors = [];
    const push = (kind, text) => window.__consoleErrors.push({ kind, text: String(text).slice(0, 300), at: location.pathname });
    const err = console.error.bind(console);
    console.error = (...a) => { push('console', a.map((x) => (x && x.message) || x).join(' ')); err(...a); };
    window.addEventListener('error', (e) => {
      const t = e.target;
      if (t && t !== window && (t.src || t.href)) push('resource', (t.tagName || '') + ' ' + (t.src || t.href));
      else push('error', e.message);
    }, true);
    window.addEventListener('unhandledrejection', (e) => push('rejection', (e.reason && e.reason.message) || e.reason));
    const f = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      const url = typeof input === 'string' ? input : input.url;
      try {
        const r = await f(input, init);
        if (!r.ok) push('fetch', r.status + ' ' + url);
        return r;
      } catch (e) {
        push('fetch', 'failed ' + url + ': ' + (e && e.message));
        throw e;
      }
    };
    return 1;
  })()`);
}

/** The console errors recorded since the last call (which clears them). */
export async function takeConsoleErrors(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
): Promise<Array<{ kind: string; text: string; at: string }>> {
  const json = (await tauriPage.evaluate(
    `JSON.stringify((window.__consoleErrors || []).splice(0))`,
  )) as string;
  return JSON.parse(json || '[]');
}
