/**
 * Playwright global teardown — kill the e2e Sensei.app and its daemon.
 *
 * Binary lifecycle (build, install, brew overlay) is owned by the Makefile;
 * there's nothing to "restore" here — the brew-installed senseid-dev formula
 * is the single source of truth for the dev binary path. After teardown,
 * the next `make app-dev` / `make app-dev-bundle` picks up where we left off.
 */
import { execFileSync } from 'child_process';
import { existsSync, readFileSync, unlinkSync } from 'fs';

const PID_FILE = '/tmp/sensei-e2e-pid';
const SOCKET   = '/tmp/tauri-playwright.sock';

export default async function globalTeardown(): Promise<void> {
  if (existsSync(PID_FILE)) {
    const raw = readFileSync(PID_FILE, 'utf8').trim();
    const pid = Number(raw);
    if (Number.isInteger(pid) && pid > 0) {
      try { process.kill(pid, 'SIGTERM'); } catch { /* already exited */ }
    } else {
      console.error(`[globalTeardown] invalid PID file contents: ${JSON.stringify(raw)}`);
    }
    unlinkSync(PID_FILE);
  }

  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid-dev'], { stdio: 'ignore' }); } catch { /* not running */ }
  try { unlinkSync(SOCKET); } catch { /* already gone */ }
}
