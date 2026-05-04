import { execFileSync, spawn } from 'child_process';
import { existsSync, symlinkSync, unlinkSync, writeFileSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

const DAEMON_REPO     = resolve(__dirname, '../../daemon');
const APP_REPO        = resolve(__dirname, '..');
const SENSEID_DEBUG   = join(DAEMON_REPO, 'target/debug/senseid');
const APP_BINARY      = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET   = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-e2e-pid';
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
  // 1. Build senseid daemon (debug)
  console.log('[globalSetup] Building senseid...');
  execFileSync('cargo', ['build', '-p', 'senseid'], {
    cwd: DAEMON_REPO,
    stdio: 'inherit',
  });

  // 2. Build Sensei.app (debug + e2e-testing feature)
  console.log('[globalSetup] Building Sensei.app...');
  execFileSync('cargo', ['tauri', 'build', '--debug', '--features', 'e2e-testing'], {
    cwd: join(APP_REPO, 'src-tauri'),
    stdio: 'inherit',
  });

  // 3. Stop any running senseid before swapping symlink
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);

  // 4. Swap symlink to debug binary
  swapSymlink(SENSEID_DEBUG, SYMLINK);

  // 5. Launch Sensei.app with dev env vars
  console.log('[globalSetup] Launching Sensei.app...');
  const proc = spawn(APP_BINARY, [], {
    env: { ...process.env, SENSEI_MODE: 'dev', SENSEI_DB_NAME: 'sensei-dev' },
    detached: true,
    stdio: 'ignore',
  });
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  // 6. Wait for tauri-plugin-playwright socket (up to 60 s)
  console.log('[globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[globalSetup] Socket ready — tests may begin.');
}
