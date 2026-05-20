// Cold-start E2E setup — verifies the health page drives itself
// from the all-services-down state through to a green observatory
// landing, with no test-driven navigation.
//
// Differences from the default globalSetup:
//   • Does NOT pre-launch senseid (the whole point is to exercise
//     the daemon_start resolver path).
//   • Drops the dev DB before launch.
//   • Stops `brew services` for postgresql@17 + ollama before launch.
//   • Does not wait for daemon port — the test observes the full
//     cold path itself.
//
// Teardown restarts the services (best-effort) so the dev box is
// returned to a working state.

import { execFileSync, spawn } from 'child_process';
import { existsSync, symlinkSync, unlinkSync, writeFileSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

const DAEMON_REPO   = resolve(__dirname, '../..');
const APP_REPO      = resolve(__dirname, '..');
const SENSEID_DEBUG = join(DAEMON_REPO, 'target/debug/senseid');
const APP_BINARY    = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET   = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-e2e-cold-pid';
const HOME     = process.env.HOME ?? '';
const SYMLINK  = join(HOME, '.local/bin/senseid');
const DB_NAME  = 'sensei_dev';

function sleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}

async function waitForSocket(socketPath: string, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (existsSync(socketPath)) return;
    await sleep(500);
  }
  throw new Error(`Timed out waiting for ${socketPath} (${timeoutMs}ms)`);
}

function swapSymlink(target: string, link: string): void {
  try { unlinkSync(link); } catch { /* did not exist */ }
  symlinkSync(target, link);
}

function tryRun(bin: string, args: string[], ignoreErrors = true): void {
  try {
    execFileSync(bin, args, { stdio: 'inherit' });
  } catch (e) {
    if (!ignoreErrors) throw e;
  }
}

export default async function globalSetup(): Promise<void> {
  if (!process.env.HOME) {
    throw new Error('$HOME is not set — cannot resolve senseid symlink path');
  }

  // 1. Build senseid + Sensei.app (same as normal E2E).
  console.log('[cold-globalSetup] Building senseid (--features dev)...');
  execFileSync('cargo', ['build', '--features', 'dev', '-p', 'senseid'], {
    cwd: DAEMON_REPO,
    stdio: 'inherit',
  });
  console.log('[cold-globalSetup] Building Sensei.app (--features dev,e2e-testing)...');
  execFileSync('cargo', ['tauri', 'build', '--debug', '--features', 'dev,e2e-testing'], {
    cwd: join(APP_REPO, 'src-tauri'),
    stdio: 'inherit',
  });

  // 2. Kill any running senseid + clean stale socket.
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // 3. Swap symlink so the daemon_start resolver finds the debug binary.
  swapSymlink(SENSEID_DEBUG, SYMLINK);

  // 4. Drop the dev DB. Postgres must be running for `dropdb` to work,
  //    so briefly start it, drop, then stop again.
  console.log('[cold-globalSetup] Briefly starting postgresql to drop dev DB...');
  tryRun('brew', ['services', 'start', 'postgresql@17']);
  // Give postgres a moment to bind.
  await sleep(2500);
  console.log(`[cold-globalSetup] Dropping ${DB_NAME}...`);
  tryRun('dropdb', ['--if-exists', DB_NAME]);

  // 5. Stop postgres + ollama so the health page sees the full failure scenario.
  console.log('[cold-globalSetup] Stopping postgresql@17 + ollama...');
  tryRun('brew', ['services', 'stop', 'postgresql@17']);
  tryRun('brew', ['services', 'stop', 'ollama']);
  await sleep(1500);

  // 6. Launch Sensei.app — same env as normal setup so SENSEI_DB_SCHEMA_PATH
  //    points the bootstrap at the local DDL instead of a GitHub fetch.
  console.log('[cold-globalSetup] Launching Sensei.app (cold)...');
  const proc = spawn(APP_BINARY, [], {
    env: {
      ...process.env,
      SENSEI_DB_SCHEMA_PATH: join(DAEMON_REPO, 'database'),
    },
    detached: true,
    stdio: 'ignore',
  });

  await new Promise<void>((resolveFn, rejectFn) => {
    proc.once('error', rejectFn);
    proc.once('spawn', resolveFn);
  });
  if (proc.pid == null) throw new Error(`Failed to get PID for ${APP_BINARY}`);
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  // 7. Wait only for the Tauri socket. Crucially, do NOT wait for the
  //    daemon port — the test verifies that the resolve walk brings it
  //    up on its own.
  console.log('[cold-globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[cold-globalSetup] Socket ready — tests may begin.');
}
