/**
 * Playwright global setup — lifecycle only, no build work.
 *
 * Building binaries is the Makefile's job:
 *   make test-app-e2e
 *     ├── app-e2e-build
 *     │     ├── install-dev   (builds + overlays senseid-dev, sensei-dev, sensei-mcp-dev into brew prefix)
 *     │     └── tauri build --debug --features dev,e2e-testing
 *     └── bun run test:e2e    (runs Playwright → this file)
 *
 * Here we only:
 *   • stop any running dev daemon and clean stale sockets
 *   • launch the e2e-built Sensei.app
 *   • wait for the Tauri IPC socket and the dev daemon port
 *
 * Running `bun run test:e2e` directly (without `make test-app-e2e`) assumes
 * the binaries and the bundle are already current. If they're not, the
 * tests will exercise stale code — always go through the Makefile.
 */
import { execFileSync, spawn } from 'child_process';
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
const PID_FILE = '/tmp/sensei-e2e-pid';
const DAEMON_PORT = 7745;

const sleep = (ms: number) => new Promise<void>(r => setTimeout(r, ms));

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
    const open = await new Promise<boolean>(r => {
      const s = createConnection({ port, host: '127.0.0.1' });
      s.once('connect', () => { s.destroy(); r(true); });
      s.once('error',   () => { s.destroy(); r(false); });
    });
    if (open) return;
    await sleep(500);
  }
  throw new Error(`Port ${port} did not open within ${timeoutMs}ms`);
}

export default async function globalSetup(): Promise<void> {
  if (!existsSync(APP_BINARY)) {
    throw new Error(
      `Sensei.app debug bundle not found at ${APP_BINARY}.\n` +
      `Build it first: \`make app-e2e-build\` (or \`make test-app-e2e\` which chains both).`,
    );
  }

  // Stop any running dev daemon — the Tauri sidecar will spawn its own.
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid-dev'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);

  // Remove stale socket from any previous run.
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  const proc = spawn(APP_BINARY, [], { detached: true, stdio: 'ignore' });
  await new Promise<void>((res, rej) => {
    proc.once('error', rej);
    proc.once('spawn', res);
  });
  if (proc.pid == null) throw new Error(`Failed to spawn ${APP_BINARY}`);
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  console.log('[globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[globalSetup] Socket ready.');

  console.log(`[globalSetup] Waiting for dev daemon on port ${DAEMON_PORT}...`);
  await waitForPort(DAEMON_PORT, 120_000);
  console.log('[globalSetup] Dev daemon ready — tests may begin.');
}
