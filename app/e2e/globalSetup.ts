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

  // 1. Build senseid daemon (debug)
  console.log('[globalSetup] Building senseid...');
  execFileSync('cargo', ['build', '-p', 'senseid'], {
    cwd: DAEMON_REPO,
    stdio: 'inherit',
  });

  // 2. Build Sensei.app (debug + e2e-testing feature)
  // VITE_SENSEI_MODE=dev is baked in at build time — the health page reads it to
  // suppress auto-advance so E2E tests can observe gate states before navigating.
  console.log('[globalSetup] Building Sensei.app...');
  execFileSync('cargo', ['tauri', 'build', '--debug', '--features', 'e2e-testing'], {
    cwd: join(APP_REPO, 'src-tauri'),
    stdio: 'inherit',
    env: { ...process.env, VITE_SENSEI_MODE: 'dev' },
  });

  // 3. Stop any running senseid before swapping symlink
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);

  // 3.5. Remove stale socket from any previous run
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // 4. Swap symlink to debug binary
  swapSymlink(SENSEID_DEBUG, SYMLINK);

  // 4.5. Start dev daemon (port 7745) — Tauri app in dev mode connects here.
  // The Tauri app's configure-assistants tests also POST directly to this port.
  console.log('[globalSetup] Starting dev daemon on port 7745...');
  const daemon = spawn(SENSEID_DEBUG, ['start'], {
    env: { ...process.env, SENSEI_MODE: 'dev', DATABASE_URL: 'postgresql://localhost:5432/sensei_dev' },
    detached: true,
    stdio: 'ignore',
  });
  daemon.unref();

  // Wait for daemon to be ready — TCP port check avoids HTTP URL literals.
  console.log('[globalSetup] Waiting for dev daemon on port 7745...');
  await waitForPort(7745, 15_000);
  console.log('[globalSetup] Dev daemon ready.');

  // 5. Launch Sensei.app with dev env vars
  console.log('[globalSetup] Launching Sensei.app...');
  const proc = spawn(APP_BINARY, [], {
    env: {
      ...process.env,
      SENSEI_MODE: 'dev',
      SENSEI_DB_NAME: 'sensei-dev',
      // Local schema path so deploy() uses the checked-out DDL instead of GitHub download
      SENSEI_DB_SCHEMA_PATH: join(DAEMON_REPO, 'database/ddl'),
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
  console.log('[globalSetup] Socket ready — tests may begin.');
}
