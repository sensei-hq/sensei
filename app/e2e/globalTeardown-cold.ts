// Cold-start E2E teardown — kills the test app + daemon and best-effort
// restarts the brew services we stopped, so the dev box returns to a
// working state.

import { execFileSync } from 'child_process';
import { existsSync, readFileSync, symlinkSync, unlinkSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

const DAEMON_REPO     = resolve(__dirname, '../..');
const SENSEID_RELEASE = join(DAEMON_REPO, 'target/release/senseid');
const PID_FILE        = '/tmp/sensei-e2e-cold-pid';
const HOME            = process.env.HOME ?? '';
const SYMLINK         = join(HOME, '.local/bin/senseid');

function swapSymlink(target: string, link: string): void {
  try { unlinkSync(link); } catch { /* did not exist */ }
  symlinkSync(target, link);
}

function tryRun(bin: string, args: string[]): void {
  try { execFileSync(bin, args, { stdio: 'inherit' }); } catch { /* best effort */ }
}

export default async function globalTeardown(): Promise<void> {
  // 1. Kill Sensei.app by saved PID.
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

  // 2. Stop any senseid the test may have spawned via the daemon_start
  //    resolver, so we don't leak a daemon process.
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }

  // 3. Restore the symlink to the release binary if one exists.
  if (existsSync(SENSEID_RELEASE)) {
    swapSymlink(SENSEID_RELEASE, SYMLINK);
  }

  // 4. Best-effort restart of the services the cold setup stopped, so
  //    a dev box doesn't get left in a stopped state after the run.
  //    If the developer had them already stopped, they'll just need to
  //    re-stop — harmless.
  tryRun('brew', ['services', 'start', 'postgresql@17']);
  tryRun('brew', ['services', 'start', 'ollama']);
}
