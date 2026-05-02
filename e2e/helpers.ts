/**
 * Shared test helpers for Playwright e2e tests.
 *
 * Manages the daemon lifecycle in dev mode (.sensei-dev/, port 7745)
 * so tests never touch production data.
 */

import { execSync, spawn, type ChildProcess } from 'child_process';
import { existsSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = parseInt(process.env.SENSEI_PORT ?? '7745', 10);
const MODE = process.env.SENSEI_MODE ?? 'dev';
const BIN = process.env.SENSEID_BIN ?? '../../target/debug/senseid';
const LOCAL_FOLDER = (process.env.LOCAL_FOLDER ?? '~/Developer').replace('~', process.env.HOME ?? '');

let daemonProcess: ChildProcess | null = null;

export { PORT, MODE, LOCAL_FOLDER };

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

/** Start the daemon in dev mode. Kills any existing instance first. */
export async function startDaemon(): Promise<void> {
  await stopDaemon();

  const bin = resolveBin();
  daemonProcess = spawn(bin, ['--mode', MODE, '--port', String(PORT)], {
    stdio: 'ignore',
    detached: true,
    env: { ...process.env, SENSEI_MODE: MODE },
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
 * Safe in browser mode too: the import resolves via Vite's dev server.
 */
export async function navigateTo(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
  route: string,
): Promise<void> {
  await tauriPage.evaluate(`
    (async function() {
      await new Promise(r => setTimeout(r, 200));
      var nav = await import('/node_modules/@sveltejs/kit/src/runtime/app/navigation.js');
      await nav.goto(${JSON.stringify(route)});
    })()
  `);
  // Give the new route's components time to mount
  await sleep(800);
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
