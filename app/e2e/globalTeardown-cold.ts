/**
 * Cold-start E2E globalTeardown — process cleanup only.
 *
 * Service restoration (brew services start postgres + ollama) is owned by
 * the Makefile's `_e2e-cold-post` target so the dev box always returns to
 * a working state, regardless of whether the test passed or threw.
 */
import { execFileSync } from 'child_process';
import { existsSync, readFileSync, unlinkSync } from 'fs';

const PID_FILE = '/tmp/sensei-e2e-cold-pid';
const SOCKET   = '/tmp/tauri-playwright.sock';

export default async function globalTeardown(): Promise<void> {
  if (existsSync(PID_FILE)) {
    const raw = readFileSync(PID_FILE, 'utf8').trim();
    const pid = Number(raw);
    if (Number.isInteger(pid) && pid > 0) {
      try { process.kill(pid, 'SIGTERM'); } catch { /* already exited */ }
    } else {
      console.error(`[cold-globalTeardown] invalid PID file contents: ${JSON.stringify(raw)}`);
    }
    unlinkSync(PID_FILE);
  }

  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid-dev'], { stdio: 'ignore' }); } catch { /* not running */ }
  try { unlinkSync(SOCKET); } catch { /* already gone */ }
}
