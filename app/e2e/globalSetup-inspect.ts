/**
 * Inspect-harness globalSetup — launch the built Sensei.app against the REAL
 * `sensei` DB for eyeballing live data (real project metrics, etc.).
 *
 * Unlike the e2e harness (globalSetup.ts), this does NOT set SENSEI_INSTANCE and
 * does NOT stop the brew `sensei` service: the app talks to the already-running
 * brew daemon on :7744 backed by the real `sensei` database. Nothing is mutated
 * — it's a read-only viewer. Requires the debug bundle (`make app-e2e-build`)
 * and a running daemon (`brew services start sensei`).
 */
import { spawn } from 'child_process';
import { existsSync, unlinkSync, writeFileSync } from 'fs';
import { createConnection } from 'net';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');
const APP_REPO = resolve(__dirname, '..');
const APP_BINARY = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-inspect-pid';
const DAEMON_PORT = 7744;

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

async function waitForSocket(path: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(path)) return;
    await sleep(500);
  }
  throw new Error(`Timed out waiting for ${path} (${timeoutMs}ms)`);
}

async function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const open = await new Promise<boolean>((r) => {
      const s = createConnection({ port, host: '127.0.0.1' });
      s.once('connect', () => { s.destroy(); r(true); });
      s.once('error', () => { s.destroy(); r(false); });
    });
    if (open) return;
    await sleep(500);
  }
  throw new Error(`Daemon port ${port} not open within ${timeoutMs}ms — is the real daemon running? \`brew services start sensei\``);
}

export default async function globalSetup(): Promise<void> {
  if (!existsSync(APP_BINARY)) {
    throw new Error(
      `Sensei.app debug bundle not found at ${APP_BINARY}.\n` +
      `Build it first: \`make app-e2e-build\`.`,
    );
  }

  // The REAL daemon (brew sensei, real DB) must already be listening. We do not
  // start/stop it — the inspector is a read-only viewer of live data.
  console.log(`[inspect] Waiting for real daemon on :${DAEMON_PORT}...`);
  await waitForPort(DAEMON_PORT, 15_000);

  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // Launch the app with NO SENSEI_INSTANCE → it resolves the real `sensei` DB /
  // data dir and uses the running brew daemon.
  console.log('[inspect] Launching Sensei.app against the real DB...');
  const proc = spawn(APP_BINARY, [], { detached: true, stdio: 'ignore', env: { ...process.env } });
  await new Promise<void>((res, rej) => {
    proc.once('error', rej);
    proc.once('spawn', res);
  });
  if (proc.pid == null) throw new Error(`Failed to spawn ${APP_BINARY}`);
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  console.log('[inspect] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  await sleep(2000); // let the SvelteKit UI boot past the health gate
  console.log('[inspect] Ready.');
}
