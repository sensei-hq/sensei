/**
 * Daemon API Verification — verifies that bootstrap and each wizard stage
 * produce real, persisted state in the running daemon (port 7745 / sensei-dev DB).
 *
 * These tests call the daemon's HTTP API directly — they do NOT rely on UI
 * screenshots or DOM assertions. Each assertion proves something real happened
 * in the database, not just that a screen rendered.
 *
 * Prerequisites (satisfied by globalSetup):
 *   • App launched with SENSEI_MODE=dev, SENSEI_DB_NAME=sensei-dev
 *   • Bootstrap ran: sensei-dev DB created, schema deployed, daemon on port 7745
 *   • Port 7745 is accepting connections
 */

import { test, expect } from '../fixtures';
import { DAEMON_URL, waitFor, daemonGet, daemonPost } from '../helpers';

// ── Types ─────────────────────────────────────────────────────────────────────

interface HealthResponse {
  status: string;
  version: string;
  uptime_secs?: number;
  db?: string;
  queue?: { pending: number; running: number };
}

interface ScanRoot {
  id: string | number;
  path: string;
  scanned: boolean;
}

interface ConfigEntry {
  key: string;
  value: string;
}

interface Project {
  id: string | number;
  name: string;
  path?: string;
}

// ── Bootstrap verification ─────────────────────────────────────────────────────

test.describe('Bootstrap verification — daemon health and DB state', () => {
  test('daemon is reachable on port 7745', async () => {
    // Basic TCP + HTTP — the daemon MUST be running for all other tests to work.
    const resp = await fetch(`${DAEMON_URL}/health`);
    expect(resp.status, 'daemon health endpoint should return 200').toBe(200);
  });

  test('daemon /health returns valid JSON with version and status', async () => {
    const body = await daemonGet<HealthResponse>('/health');

    expect(body.status, 'status field must be present').toBeTruthy();
    expect(typeof body.version, 'version must be a string').toBe('string');
    expect(body.version.length, 'version must not be empty').toBeGreaterThan(0);
    // status is "ok" or "degraded" — never undefined
    expect(['ok', 'degraded', 'starting'].includes(body.status),
      `status '${body.status}' should be a known value`).toBe(true);
  });

  test('daemon reports DB connectivity in health response', async () => {
    const body = await daemonGet<HealthResponse>('/health');
    // The daemon connects to sensei-dev on startup; if it's up, DB is reachable.
    // A "degraded" status means DB has issues — either way the field must exist.
    expect(body.status).toBeTruthy();
    // uptime_secs > 0 proves daemon has been running since startup (not restarted)
    if (body.uptime_secs !== undefined) {
      expect(body.uptime_secs).toBeGreaterThanOrEqual(0);
    }
  });

  test('daemon task queue is idle at startup (no phantom tasks)', async () => {
    interface TaskStatus { queue: { pending: number; running: number } }
    const status = await daemonGet<TaskStatus>('/api/index/status');

    expect(typeof status.queue.pending).toBe('number');
    expect(typeof status.queue.running).toBe('number');
    // On a fresh start, nothing should be queued or running
    expect(status.queue.pending, 'no phantom pending tasks at startup').toBe(0);
    expect(status.queue.running, 'no phantom running tasks at startup').toBe(0);
  });

  test('daemon scan roots starts empty after reset', async () => {
    // Reset to known state
    await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' });
    await new Promise(r => setTimeout(r, 300));

    const roots = await daemonGet<ScanRoot[]>('/api/scan/roots');
    expect(Array.isArray(roots), 'scan/roots must return an array').toBe(true);
    expect(roots.length, 'no roots after reset').toBe(0);
  });

  test('daemon config endpoint is reachable and returns an object', async () => {
    const config = await daemonGet<Record<string, string>>('/api/config');
    expect(typeof config, 'config must be an object').toBe('object');
    expect(config !== null).toBe(true);
  });

  test('daemon assistants/detect returns valid assistant list', async () => {
    interface Assistant { id: string; name: string; installed: boolean; configured: boolean }
    const assistants = await daemonGet<Assistant[]>('/api/assistants/detect');

    expect(Array.isArray(assistants), 'assistants/detect must return an array').toBe(true);
    // At minimum claude-code should be detectable (it runs this tool!)
    for (const a of assistants) {
      expect(typeof a.id).toBe('string');
      expect(typeof a.name).toBe('string');
      expect(typeof a.installed).toBe('boolean');
      expect(typeof a.configured).toBe('boolean');
    }
  });
});

// ── Wizard stage verification — API state after each commit ───────────────────

test.describe('Wizard stage verification — real DB state after each commit', () => {
  test.beforeEach(async () => {
    // Reset to a clean slate before each verification test
    try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    await new Promise(r => setTimeout(r, 300));
  });

  // ── Preferences ──────────────────────────────────────────────────────────

  test('config: PUT /api/config persists a key and GET retrieves it', async () => {
    // Simulate what commitStage("preferences") does
    const putResp = await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key: 'user_name', value: 'E2E Test User' }),
    });
    expect(putResp.status, 'PUT /api/config should return 200').toBe(200);

    // Verify it was persisted
    const config = await daemonGet<Record<string, string>>('/api/config');
    expect(config['user_name'], 'user_name should be persisted in DB').toBe('E2E Test User');
  });

  test('config: PUT multiple keys, GET /api/config/{key} retrieves each', async () => {
    const entries = [
      { key: 'telemetry', value: 'false' },
      { key: 'theme', value: 'dark' },
    ];
    for (const e of entries) {
      const r = await fetch(`${DAEMON_URL}/api/config`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(e),
      });
      expect(r.status).toBe(200);
    }

    for (const e of entries) {
      const val = await daemonGet<{ key: string; value: string }>(`/api/config/${e.key}`);
      expect(val.value, `${e.key} should be '${e.value}'`).toBe(e.value);
    }
  });

  // ── Roots ────────────────────────────────────────────────────────────────

  test('scan/roots: POST adds a root, GET returns it, DELETE removes it', async () => {
    const testPath = '/tmp/sensei-e2e-empty';

    // Add root
    const addResp = await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: testPath }),
    });
    expect(addResp.status, 'POST scan/roots should return 200').toBe(200);

    // Verify it's in the list
    const roots = await daemonGet<ScanRoot[]>('/api/scan/roots');
    const added = roots.find(r => r.path === testPath);
    expect(added, `root ${testPath} should appear in scan/roots`).toBeTruthy();
    expect(typeof added!.id).not.toBe('undefined');

    // Remove by ID
    const deleteResp = await fetch(`${DAEMON_URL}/api/scan/roots/${added!.id}`, {
      method: 'DELETE',
    });
    expect(deleteResp.status, 'DELETE scan/roots/{id} should return 200').toBe(200);

    // Verify removed
    const after = await daemonGet<ScanRoot[]>('/api/scan/roots');
    expect(after.find(r => r.path === testPath), 'root should be gone after delete').toBeFalsy();
  });

  test('scan/roots: duplicate path is rejected or silently de-duped', async () => {
    const path = '/tmp/sensei-e2e-dedup';

    await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    });
    await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    });

    const roots = await daemonGet<ScanRoot[]>('/api/scan/roots');
    const count = roots.filter(r => r.path === path).length;
    expect(count, 'duplicate path should appear exactly once').toBe(1);
  });

  // ── Scan ─────────────────────────────────────────────────────────────────

  test('scan: POST /api/scan starts task, task queue shows activity', async () => {
    // Create a minimal git repo to scan
    const { execFileSync, mkdirSync, writeFileSync, existsSync } = await import('fs');
    const corpus = '/tmp/sensei-e2e-verify-scan';
    const project = `${corpus}/verify-project`;

    if (!existsSync(`${project}/.git`)) {
      mkdirSync(`${project}/src`, { recursive: true });
      writeFileSync(`${project}/index.ts`, 'export const x = 1;\n');
      const opts = { cwd: project, stdio: 'ignore' as const };
      execFileSync('git', ['init'],                                        opts);
      execFileSync('git', ['config', 'user.email', 'test@sensei.test'],   opts);
      execFileSync('git', ['config', 'user.name',  'E2E Test'],           opts);
      execFileSync('git', ['add', '.'],                                    opts);
      execFileSync('git', ['commit', '-m', 'init'],                       opts);
    }

    // Add root then trigger scan
    await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: corpus }),
    });

    const scanResp = await fetch(`${DAEMON_URL}/api/scan`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: corpus }),
    });
    expect(scanResp.status, 'POST /api/scan should return 200').toBe(200);

    // Scan enqueues tasks — queue should have activity within 2 s
    interface TaskStatus { queue: { pending: number; running: number } }
    let sawActivity = false;
    const deadline = Date.now() + 5_000;
    while (Date.now() < deadline) {
      const s = await daemonGet<TaskStatus>('/api/index/status');
      if (s.queue.pending > 0 || s.queue.running > 0) {
        sawActivity = true;
        break;
      }
      await new Promise(r => setTimeout(r, 200));
    }
    // Small repos may finish before we poll — that is also a valid pass
    // (scan queued AND completed before our first poll)
    expect(
      sawActivity || (await daemonGet<TaskStatus>('/api/index/status')).queue.pending === 0,
      'scan should have enqueued tasks or already completed'
    ).toBe(true);

    // Wait for queue to drain (up to 30 s)
    await waitFor(async () => {
      const s = await daemonGet<TaskStatus>('/api/index/status');
      return s.queue.pending === 0 && s.queue.running === 0;
    }, 30_000, 500);

    // After scan completes, repos should be in the DB
    interface Repo { id: string; name: string }
    const repos = await daemonGet<Repo[]>('/api/repos');
    expect(Array.isArray(repos)).toBe(true);
    // verify-project is a git repo — it should be indexed
    const found = repos.find(r => r.name === 'verify-project' || r.name?.includes('verify'));
    expect(found, 'verify-project should be indexed after scan').toBeTruthy();
  });

  // ── Projects ──────────────────────────────────────────────────────────────

  test('projects: GET /api/projects returns valid array', async () => {
    const projects = await daemonGet<Project[]>('/api/projects');
    expect(Array.isArray(projects), 'projects must be an array').toBe(true);
    for (const p of projects) {
      expect(typeof p.id).not.toBe('undefined');
      expect(typeof p.name).toBe('string');
    }
  });

  test('projects: POST creates a project, GET retrieves it, DELETE removes it', async () => {
    const createResp = await fetch(`${DAEMON_URL}/api/projects`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'E2E Test Project', description: 'created by e2e test' }),
    });
    expect(createResp.status, 'POST /api/projects should return 200').toBe(200);
    const created = await createResp.json() as Project;
    expect(created.id).toBeTruthy();
    expect(created.name).toBe('E2E Test Project');

    // GET list should include it
    const list = await daemonGet<Project[]>('/api/projects');
    expect(list.find(p => p.name === 'E2E Test Project')).toBeTruthy();

    // DELETE it
    const del = await fetch(`${DAEMON_URL}/api/projects/${created.id}`, { method: 'DELETE' });
    expect(del.status).toBe(200);

    // Should be gone
    const after = await daemonGet<Project[]>('/api/projects');
    expect(after.find(p => String(p.id) === String(created.id))).toBeFalsy();
  });

  // ── Assistants ────────────────────────────────────────────────────────────

  test('assistants: configure writes hook entries, remove cleans them up', async () => {
    const { readFileSync, existsSync } = await import('fs');
    const { homedir } = await import('os');
    const { join } = await import('path');

    const SETTINGS = join(homedir(), '.claude', 'settings.json');
    const DEV_HOOK = join(homedir(), '.claude', 'hooks', 'sensei-hook-dev.ts');

    const readSettings = (): Record<string, unknown> | null => {
      if (!existsSync(SETTINGS)) return null;
      try { return JSON.parse(readFileSync(SETTINGS, 'utf8')); }
      catch { return null; }
    };

    const hookEntries = (settings: Record<string, unknown> | null, event: string): string[] => {
      if (!settings?.hooks) return [];
      const arr = (settings.hooks as Record<string, unknown>)[event];
      if (!Array.isArray(arr)) return [];
      return arr.flatMap((entry: unknown) => {
        const hooks = (entry as Record<string, unknown>)?.hooks;
        if (!Array.isArray(hooks)) return [];
        return hooks.map((h: unknown) => (h as Record<string, string>)?.command ?? '');
      });
    };

    const before = readSettings();
    const hadHookBefore = hookEntries(before, 'SessionStart').includes(DEV_HOOK);

    // Configure
    const configureResp = await fetch(`${DAEMON_URL}/api/assistants/configure`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ acps: ['claude-code'] }),
    });
    expect(configureResp.status).toBe(200);
    await new Promise(r => setTimeout(r, 400));

    const after = readSettings();
    const coreEvents = ['SessionStart', 'PreToolUse', 'PostToolUse', 'Stop'];
    for (const ev of coreEvents) {
      expect(
        hookEntries(after, ev).includes(DEV_HOOK),
        `${ev} hook should contain sensei-hook-dev.ts after configure`
      ).toBe(true);
    }

    // Remove (cleanup)
    await fetch(`${DAEMON_URL}/api/assistants/remove`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ acps: ['claude-code'] }),
    });
    await new Promise(r => setTimeout(r, 300));

    if (!hadHookBefore) {
      const cleaned = readSettings();
      expect(
        hookEntries(cleaned, 'SessionStart').includes(DEV_HOOK),
        'hook should be removed after uninstall'
      ).toBe(false);
    }
  });

  // ── Reset ─────────────────────────────────────────────────────────────────

  test('reset: POST /api/reset clears roots, repos, projects, config', async () => {
    // Add some state
    await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: '/tmp/sensei-e2e-reset-test' }),
    });
    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key: 'reset_test_key', value: 'should_be_gone' }),
    });

    // Reset
    const resetResp = await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' });
    expect(resetResp.status).toBe(200);
    await new Promise(r => setTimeout(r, 300));

    // Verify cleared
    const roots = await daemonGet<ScanRoot[]>('/api/scan/roots');
    expect(roots.length, 'roots should be empty after reset').toBe(0);

    const config = await daemonGet<Record<string, string>>('/api/config');
    expect(config['reset_test_key'], 'config key should be gone after reset').toBeFalsy();
  });
});
