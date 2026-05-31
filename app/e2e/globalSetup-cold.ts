/**
 * Cold-start E2E globalSetup — lifecycle only, no build work.
 *
 * Building binaries is the Makefile's job:
 *   make test-app-e2e-cold
 *     ├── app-e2e-build       (install-debug + tauri build --features e2e-testing)
 *     ├── _e2e-cold-pre       (drops sensei_e2e, stops postgres + ollama)
 *     ├── bun run test:e2e:cold
 *     └── _e2e-cold-post      (restarts postgres + ollama, always)
 *
 * Here we only:
 *   • stop any running daemon and clean stale sockets
 *   • launch the e2e-built Sensei.app with SENSEI_INSTANCE=e2e so the
 *     daemon uses sensei_e2e + ~/.sensei-e2e/ (NOT the user's real DB)
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
const INSTANCE = 'e2e';

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

  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // SENSEI_DDL_DIR points bootstrap at the local DDL so the database
  // resolver doesn't need GitHub access during the cold-start test.
  // SENSEI_INSTANCE=e2e isolates the DB + data dir from the user's real
  // install.
  const proc = spawn(APP_BINARY, [], {
    env: {
      ...process.env,
      SENSEI_DDL_DIR:   join(REPO_ROOT, 'database'),
      SENSEI_INSTANCE:  INSTANCE,
    },
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
  console.log(`[cold-globalSetup] Socket ready (instance=${INSTANCE}) — tests may begin.`);
}
