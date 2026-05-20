// Cold-start E2E globalTeardown — only the playwright-intrinsic
// app/process cleanup. The Makefile's `_e2e-cold-post` target handles
// restarting brew services (postgres + ollama) so the dev box returns
// to a working state even if the test failed.

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
}
