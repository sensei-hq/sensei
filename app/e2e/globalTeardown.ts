import { execFileSync } from 'child_process';
import { existsSync, readFileSync, symlinkSync, unlinkSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

const DAEMON_REPO     = resolve(__dirname, '../../daemon');
const SENSEID_RELEASE = join(DAEMON_REPO, 'target/release/senseid');
const PID_FILE        = '/tmp/sensei-e2e-pid';
const HOME            = process.env.HOME ?? '';
const SYMLINK         = join(HOME, '.local/bin/senseid');

function swapSymlink(target: string, link: string): void {
  try { unlinkSync(link); } catch { /* did not exist */ }
  symlinkSync(target, link);
}

export default async function globalTeardown(): Promise<void> {
  // 1. Kill Sensei.app by saved PID
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

  // 2. Stop senseid daemon
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }

  // 3. Restore symlink to release binary
  if (existsSync(SENSEID_RELEASE)) {
    swapSymlink(SENSEID_RELEASE, SYMLINK);
  }
}
