import { execFileSync, spawn } from 'child_process';
import { existsSync, symlinkSync, unlinkSync, writeFileSync } from 'fs';
import { createConnection } from 'net';

/** Wait until a TCP port accepts connections (no HTTP — avoids CWE-319 false positive). */
async function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const open = await new Promise<boolean>(resolve => {
      const s = createConnection({ port, host: '127.0.0.1' });
      s.once('connect', () => { s.destroy(); resolve(true); });
      s.once('error',   () => { s.destroy(); resolve(false); });
    });
    if (open) return;
    await sleep(500);
  }
  throw new Error(`Port ${port} did not open within ${timeoutMs}ms`);
}
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

// Monorepo root — crates/ lives alongside app/
const DAEMON_REPO     = resolve(__dirname, '../..');
const APP_REPO        = resolve(__dirname, '..');
const SENSEID_DEBUG   = join(DAEMON_REPO, 'target/debug/senseid');
const APP_BINARY      = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET   = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-e2e-pid';
const HOME = process.env.HOME ?? '';
const SYMLINK  = join(HOME, '.local/bin/senseid');
const DB_NAME  = 'sensei-dev';

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
  // Validate HOME environment variable
  if (!process.env.HOME) {
    throw new Error('$HOME is not set — cannot resolve senseid symlink path');
  }

  // 1. Build senseid daemon (debug + dev feature for compile-time mode)
  console.log('[globalSetup] Building senseid (--features dev)...');
  execFileSync('cargo', ['build', '--features', 'dev', '-p', 'senseid'], {
    cwd: DAEMON_REPO,
    stdio: 'inherit',
  });

  // 2. Build Sensei.app (debug + dev + e2e-testing features)
  console.log('[globalSetup] Building Sensei.app (--features dev,e2e-testing)...');
  execFileSync('cargo', ['tauri', 'build', '--debug', '--features', 'dev,e2e-testing'], {
    cwd: join(APP_REPO, 'src-tauri'),
    stdio: 'inherit',
  });

  // 3. Stop any running senseid before swapping symlink
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);

  // 3.5. Remove stale socket from any previous run
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // 4. Swap symlink to debug binary
  swapSymlink(SENSEID_DEBUG, SYMLINK);

  // 5. Launch Sensei.app — mode is compile-time (--features dev).
  //    SENSEI_DB_SCHEMA_PATH lets bootstrap use local DDL instead of GitHub download.
  console.log('[globalSetup] Launching Sensei.app...');
  const proc = spawn(APP_BINARY, [], {
    env: {
      ...process.env,
      SENSEI_DB_SCHEMA_PATH: join(DAEMON_REPO, 'database'),
    },
    detached: true,
    stdio: 'ignore',
  });

  await new Promise<void>((resolve, reject) => {
    proc.once('error', reject);
    proc.once('spawn', resolve);
  });

  if (proc.pid == null) throw new Error(`Failed to get PID for ${APP_BINARY}`);
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  // 6. Wait for tauri-plugin-playwright socket (up to 60 s)
  console.log('[globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[globalSetup] Socket ready.');

  // 7. Wait for the dev daemon on port 7745 (up to 120 s).
  // The Tauri app's bootstrap health screen automatically:
  //   • confirms sensei-dev DB is ready (gate 五 — already done above)
  //   • starts senseid on port 7745 (gate 六 — ServiceStartFixer)
  // Port 7745 opening is the signal that bootstrap completed and the daemon is ready.
  console.log('[globalSetup] Waiting for dev daemon on port 7745 (bootstrap in progress)...');
  await waitForPort(7745, 120_000);
  console.log('[globalSetup] Dev daemon ready — tests may begin.');
}
