/**
 * Playwright global setup — lifecycle only, no build work.
 *
 * Building binaries is the Makefile's job:
 *   make test-app-e2e
 *     ├── app-e2e-build
 *     │     ├── install-debug   (builds + overlays senseid/sensei/sensei-mcp into brew prefix)
 *     │     └── tauri build --debug --features e2e-testing
 *     └── bun run test:e2e      (runs Playwright → this file)
 *
 * Here we only:
 *   • stop any running daemon and clean stale sockets
 *   • launch the e2e-built Sensei.app with SENSEI_INSTANCE=e2e so the
 *     daemon talks to a throwaway `sensei_e2e` DB in `~/.sensei-e2e/`
 *     instead of the user's real `sensei` DB
 *   • wait for the Tauri IPC socket and the daemon port
 *
 * Running `bun run test:e2e` directly (without `make test-app-e2e`) assumes
 * the binaries and the bundle are already current. If they're not, the
 * tests will exercise stale code — always go through the Makefile.
 */
import { execFileSync, spawn } from 'child_process';
import { existsSync, unlinkSync, writeFileSync } from 'fs';
import { createConnection } from 'net';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');
const APP_REPO = resolve(__dirname, '..');
const APP_BINARY = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-e2e-pid';
const DAEMON_PORT = 7744;
const INSTANCE  = 'e2e';

const sleep = (ms: number) => new Promise<void>(r => setTimeout(r, ms));

async function waitForSocket(path: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(path)) return;
    await sleep(500);
  }
  throw new Error(`Timed out waiting for ${path} (${timeoutMs}ms)`);
}

async function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const open = await new Promise<boolean>(r => {
      const s = createConnection({ port, host: '127.0.0.1' });
      s.once('connect', () => { s.destroy(); r(true); });
      s.once('error',   () => { s.destroy(); r(false); });
    });
    if (open) return;
    await sleep(500);
  }
  throw new Error(`Port ${port} did not open within ${timeoutMs}ms`);
}

/** Stop the brew-managed sensei service so launchd can't race-respawn a
 *  prod-DB daemon between our pkill and the Tauri shell bringing up its
 *  own instance-tagged one. Restored in globalTeardown. */
function stopBrewSensei(): void {
  try { execFileSync('/opt/homebrew/bin/brew', ['services', 'stop', 'sensei'], { stdio: 'ignore' }); }
  catch { /* brew may be elsewhere, or service may not exist */ }
}

export default async function globalSetup(): Promise<void> {
  if (!existsSync(APP_BINARY)) {
    throw new Error(
      `Sensei.app debug bundle not found at ${APP_BINARY}.\n` +
      `Build it first: \`make app-e2e-build\` (or \`make test-app-e2e\` which chains both).`,
    );
  }

  // 1. Stop the brew service FIRST. launchd's keep-alive policy will
  //    otherwise auto-respawn the prod daemon between our pkill below
  //    and the Tauri shell coming up with SENSEI_INSTANCE=e2e — leaving
  //    the .app talking to a prod-DB daemon that doesn't know it's
  //    supposed to be isolated. globalTeardown restores it.
  console.log('[globalSetup] Stopping brew sensei service (race-prevention)...');
  stopBrewSensei();

  // 2. Kill any senseid that survived the brew stop (e.g. a manual spawn).
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);

  // 3. Remove stale socket from any previous run.
  try { unlinkSync(SOCKET); } catch { /* did not exist */ }

  // 4. Spawn the .app with SENSEI_INSTANCE=e2e so the daemon resolves DB
  //    + data dir to a throwaway pair (sensei_e2e / ~/.sensei-e2e/). On
  //    macOS, Tauri inherits the spawning environment for child process
  //    spawns, and bootstrap's daemon_start resolver now ALSO passes
  //    --instance=e2e on the senseid CLI as a second line of defence.
  //    SENSEI_DDL_DIR points bootstrap at the repo DDL so the fresh e2e DB
  //    gets the CURRENT schema (incl. additive columns not yet in the released
  //    bundle — the bundle can lag the code between bumps). Mirrors
  //    globalSetup-cold; without it, tests exercising new columns 500 on a
  //    "column does not exist" from the stale bundled schema.
  const proc = spawn(APP_BINARY, [], {
    detached: true,
    stdio: 'ignore',
    env: {
      ...process.env,
      SENSEI_DDL_DIR: join(APP_REPO, '..', 'database'),
      SENSEI_INSTANCE: INSTANCE,
    },
  });
  await new Promise<void>((res, rej) => {
    proc.once('error', rej);
    proc.once('spawn', res);
  });
  if (proc.pid == null) throw new Error(`Failed to spawn ${APP_BINARY}`);
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  console.log('[globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[globalSetup] Socket ready.');

  console.log(`[globalSetup] Waiting for daemon on port ${DAEMON_PORT} (instance=${INSTANCE})...`);
  // The daemon compiles/loads the embedded model before it binds the port; a
  // cold box right after a build can take >2min, so allow generous headroom
  // (a genuine hang is still bounded by Playwright's own run timeout).
  await waitForPort(DAEMON_PORT, 240_000);

  // 5. Authoritative DB-isolation check — fetch /health from the daemon
  //    actually bound to the port and confirm it's on the expected
  //    sensei_<instance> DB. If anything raced us (launchd, a stray
  //    senseid spawn, a misconfigured Tauri build), the wrong daemon
  //    is on the port and we must NOT proceed — the tests would write
  //    to the prod DB and corrupt the user's data.
  const expectedDb = `sensei_${INSTANCE}`;
  // Authoritative DB-isolation check via /health — loopback-only daemon,
  // same URL pattern as DAEMON_URL elsewhere in e2e/helpers.ts.
  const { DAEMON_URL } = await import('./helpers.js');
  const health = await fetch(`${DAEMON_URL}/health`).then(r => r.json());
  if (health.dbName !== expectedDb) {
    throw new Error(
      `[globalSetup] DB ISOLATION CHECK FAILED. ` +
      `Daemon on port ${DAEMON_PORT} is connected to "${health.dbName}", ` +
      `expected "${expectedDb}". Refusing to run tests against the wrong DB.`,
    );
  }
  console.log(`[globalSetup] Daemon DB verified: ${health.dbName} — tests may begin.`);
}
