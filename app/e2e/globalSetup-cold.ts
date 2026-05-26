/**
 * Cold-start E2E globalSetup — lifecycle only, no build work.
 *
 * Building binaries is the Makefile's job:
 *   make test-app-e2e-cold
 *     ├── app-e2e-build       (install-dev + tauri build --features dev,e2e-testing)
 *     ├── _e2e-cold-pre       (drops sensei_dev, stops postgres + ollama)
 *     ├── bun run test:e2e:cold
 *     └── _e2e-cold-post      (restarts postgres + ollama, always)
 *
 * Here we only:
 *   • stop any running dev daemon and clean stale sockets
 *   • launch the e2e-built Sensei.app
 *   • wait for the Tauri IPC socket (NOT the daemon — the test observes
 *     the resolver bringing it up)
 */
import { execFileSync, spawn } from 'child_process';
import { existsSync, unlinkSync, writeFileSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');
const REPO_ROOT = resolve(__dirname, '../..');
const APP_REPO = resolve(__dirname, '..');
const APP_BINARY = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-e2e-cold-pid';

const sleep = (ms: number) => new Promise<void>(r => setTimeout(r, ms));

async function waitForSocket(path: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(path)) return;
    await sleep(500);
  }
  throw new Error(`Timed out waiting for ${path} (${timeoutMs}ms)`);
}

export default async function globalSetup(): Promise<void> {
  if (!existsSync(APP_BINARY)) {
    throw new Error(
      `Sensei.app debug bundle not found at ${APP_BINARY}.\n` +
      `Build it first: \`make app-e2e-build\` (or \`make test-app-e2e-cold\`).`,
    );
  }

  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid-dev'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // SENSEI_DB_SCHEMA_PATH points bootstrap at local DDL so the database
  // resolver doesn't need GitHub access during the cold-start test.
  const proc = spawn(APP_BINARY, [], {
    env: { ...process.env, SENSEI_DB_SCHEMA_PATH: join(REPO_ROOT, 'database') },
    detached: true,
    stdio: 'ignore',
  });
  await new Promise<void>((res, rej) => {
    proc.once('error', rej);
    proc.once('spawn', res);
  });
  if (proc.pid == null) throw new Error(`Failed to spawn ${APP_BINARY}`);
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  console.log('[cold-globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[cold-globalSetup] Socket ready — tests may begin.');
}
