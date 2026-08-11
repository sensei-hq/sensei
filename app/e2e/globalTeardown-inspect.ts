/**
 * Inspect-harness globalTeardown — stop the app we launched, leave the brew
 * `sensei` service (and the real DB) exactly as we found it.
 */
import { existsSync, readFileSync, unlinkSync } from 'fs';

const PID_FILE = '/tmp/sensei-inspect-pid';
const SOCKET = '/tmp/tauri-playwright.sock';

export default async function globalTeardown(): Promise<void> {
  try {
    if (existsSync(PID_FILE)) {
      const pid = parseInt(readFileSync(PID_FILE, 'utf8').trim(), 10);
      if (pid) {
        try { process.kill(pid, 'SIGTERM'); } catch { /* already gone */ }
      }
      unlinkSync(PID_FILE);
    }
  } catch { /* best effort */ }
  try { unlinkSync(SOCKET); } catch { /* gone */ }
  // Intentionally NOT touching `brew services` — the real daemon keeps running.
}
