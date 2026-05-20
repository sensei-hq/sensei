// Cold-start E2E globalSetup — handles only the playwright-intrinsic
// app lifecycle (build, launch, wait for IPC socket).
//
// The destructive parts (drop sensei_dev, brew services stop postgres
// & ollama, brew services start on teardown) live in the Makefile
// target `test-app-e2e-cold`. Invoke via `make test-app-e2e-cold`
// from the repo root — running `bun run test:e2e:cold` directly will
// NOT do the service teardown and the resolvers will probably see
// services already up.

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

export default async function globalSetup(): Promise<void> {
  if (!process.env.HOME) {
    throw new Error('$HOME is not set — cannot resolve senseid symlink path');
  }

  // 1. Build senseid + Sensei.app. The Makefile's install-dev dependency
  //    on test-app-e2e-cold builds senseid + sensei into ~/.local/bin
  //    (release-mode); these debug builds are what the test exercises.
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

  // 4. Launch Sensei.app — SENSEI_DB_SCHEMA_PATH points the bootstrap at
  //    the local DDL so the database resolver doesn't need GitHub access.
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

  // 5. Wait only for the Tauri socket. We deliberately do NOT wait for
  //    the daemon port — the test observes the resolver bringing it up.
  console.log('[cold-globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[cold-globalSetup] Socket ready — tests may begin.');
}
