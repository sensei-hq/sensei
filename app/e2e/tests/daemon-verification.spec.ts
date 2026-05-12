/**
 * Daemon API Verification — verifies that bootstrap and each wizard stage
 * produce real, persisted state in the running daemon (port 7745 / sensei-dev DB).
 *
 * These tests call the daemon's HTTP API directly — they do NOT rely on UI
 * screenshots or DOM assertions. Each assertion proves something real happened
 * in the database, not just that a screen rendered.
 *
 * Prerequisites (satisfied by globalSetup):
 *   • sensei-dev DB dropped and recreated via `dbd apply` — guaranteed clean slate
 *   • App built with --features dev (compile-time: port 7745, sensei_dev DB)
 *   • Bootstrap ran: schema confirmed, daemon started on port 7745
 *   • Port 7745 is accepting connections
 *
 * Test design:
 *   • No shared reset between tests — globalSetup gives one clean DB per run
 *   • Each test uses unique identifiers (timestamps) to avoid collisions
 *   • Tests that add state also remove it in cleanup
 */

import { test, expect } from '../fixtures';
import { DAEMON_URL, waitFor, daemonGet } from '../helpers';

// ── Types ─────────────────────────────────────────────────────────────────────

interface HealthResponse {
  status: string;
  version: string;
  uptime_secs?: number;
}

interface ScanRoot {
  id: string;
  path: string;
  status?: string;
}

interface Project {
  id: string;
  name: string;
}

// ── Bootstrap verification ─────────────────────────────────────────────────────

test.describe('Bootstrap verification — daemon health and DB state', () => {
  test('daemon is reachable on port 7745', async () => {
    const resp = await fetch(`${DAEMON_URL}/health`);
    expect(resp.status, 'daemon health endpoint should return 200').toBe(200);
  });

  test('daemon /health returns valid JSON with version and status', async () => {
    const body = await daemonGet<HealthResponse>('/health');

    expect(body.status, 'status field must be present').toBeTruthy();
    expect(typeof body.version, 'version must be a string').toBe('string');
    expect(body.version.length, 'version must not be empty').toBeGreaterThan(0);
    // daemon returns "healthy" or "degraded"
    expect(['healthy', 'degraded', 'starting'].includes(body.status),
      `status '${body.status}' should be a known value`).toBe(true);
  });

  test('daemon reports uptime in health response', async () => {
    const body = await daemonGet<HealthResponse>('/health');
    if (body.uptime_secs !== undefined) {
      expect(body.uptime_secs).toBeGreaterThanOrEqual(0);
    }
  });

  test('daemon task queue is idle at startup (no phantom tasks)', async () => {
    interface TaskStatus { queue: { pending: number; running: number } }
    const status = await daemonGet<TaskStatus>('/api/index/status');

    expect(typeof status.queue.pending).toBe('number');
    expect(typeof status.queue.running).toBe('number');
    expect(status.queue.pending, 'no phantom pending tasks at startup').toBe(0);
    expect(status.queue.running, 'no phantom running tasks at startup').toBe(0);
  });

  test('daemon scan roots starts empty (fresh DB from globalSetup)', async () => {
    const roots = await daemonGet<ScanRoot[]>('/api/scan/roots');
    expect(Array.isArray(roots), 'scan/roots must return an array').toBe(true);
    expect(roots.length, 'no roots in freshly deployed DB').toBe(0);
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
    for (const a of assistants) {
      expect(typeof a.id).toBe('string');
      expect(typeof a.name).toBe('string');
      expect(typeof a.installed).toBe('boolean');
      expect(typeof a.configured).toBe('boolean');
    }
  });
});

// ── Wizard stage verification — real DB state after each operation ─────────────

test.describe('Wizard stage verification — real DB state after each commit', () => {
  // Use timestamp-based unique IDs so parallel/sequential tests don't collide.
  // No beforeEach reset — globalSetup already gave us a clean DB.
  const RUN = Date.now();

  // ── Config ───────────────────────────────────────────────────────────────

  test('config: PUT /api/config persists a key and GET retrieves it', async () => {
    const key = `e2e_test_key_${RUN}`;

    // PUT /api/config expects a JSON object: { "key": "value" }
    const putResp = await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ [key]: 'test-value' }),
    });
    expect(putResp.status, 'PUT /api/config should return 200').toBe(200);

    // GET /api/config returns HashMap<String, String> as JSON object
    const config = await daemonGet<Record<string, string>>('/api/config');
    expect(config[key], `${key} should be persisted in DB`).toBe('test-value');

    // Cleanup
    await fetch(`${DAEMON_URL}/api/config/${key}`, { method: 'DELETE' });
  });

  test('config: PUT multiple keys, GET /api/config/{key} retrieves each', async () => {
    const k1 = `e2e_telemetry_${RUN}`;
    const k2 = `e2e_theme_${RUN}`;

    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ [k1]: 'false', [k2]: 'dark' }),
    });

    const tel = await daemonGet<{ key: string; value: string }>(`/api/config/${k1}`);
    expect(tel.value, `${k1} should be 'false'`).toBe('false');

    const theme = await daemonGet<{ key: string; value: string }>(`/api/config/${k2}`);
    expect(theme.value, `${k2} should be 'dark'`).toBe('dark');

    // Cleanup
    await fetch(`${DAEMON_URL}/api/config/${k1}`, { method: 'DELETE' });
    await fetch(`${DAEMON_URL}/api/config/${k2}`, { method: 'DELETE' });
  });

  // ── Scan roots ────────────────────────────────────────────────────────────

  test('scan/roots: POST adds a root, GET returns it, DELETE removes it', async () => {
    const testPath = `/tmp/sensei-e2e-root-${RUN}`;

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

    // Remove by ID
    const deleteResp = await fetch(`${DAEMON_URL}/api/scan/roots/${added!.id}`, { method: 'DELETE' });
    expect(deleteResp.status, 'DELETE scan/roots/{id} should return 200').toBe(200);

    // Verify removed
    const after = await daemonGet<ScanRoot[]>('/api/scan/roots');
    expect(after.find(r => r.path === testPath), 'root should be gone after delete').toBeFalsy();
  });

  test('scan/roots: duplicate path is rejected or silently de-duped', async () => {
    const path = `/tmp/sensei-e2e-dedup-${RUN}`;

    const r1 = await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    });
    await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    });

    const roots = await daemonGet<ScanRoot[]>('/api/scan/roots');
    const matches = roots.filter(r => r.path === path);
    expect(matches.length, 'duplicate path should appear exactly once').toBe(1);

    // Cleanup
    const id = (await r1.json() as { id: string }).id;
    await fetch(`${DAEMON_URL}/api/scan/roots/${id}`, { method: 'DELETE' });
  });

  // ── Scan ─────────────────────────────────────────────────────────────────

  test('scan: POST /api/scan starts task, queue drains, repo is indexed', async () => {
    const { execFileSync } = await import('child_process');
    const { mkdirSync, writeFileSync, existsSync } = await import('fs');

    const corpus = `/tmp/sensei-e2e-scan-${RUN}`;
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

    // Add root
    const addResp = await fetch(`${DAEMON_URL}/api/scan/roots`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: corpus }),
    });
    const { id: rootId } = await addResp.json() as { id: string };

    // Trigger scan — body uses "root" not "path"
    const scanResp = await fetch(`${DAEMON_URL}/api/scan`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ root: corpus }),
    });
    expect(scanResp.status, 'POST /api/scan should return 200').toBe(200);

    // Wait for queue to drain (up to 30 s)
    interface TaskStatus { queue: { pending: number; running: number } }
    await waitFor(async () => {
      const s = await daemonGet<TaskStatus>('/api/index/status');
      return s.queue.pending === 0 && s.queue.running === 0;
    }, 30_000, 500);

    // After scan completes, the repo should appear in /api/repos
    interface Repo { id: string; name: string }
    const repos = await daemonGet<Repo[]>('/api/repos');
    expect(Array.isArray(repos)).toBe(true);
    const found = repos.find(r => r.name === 'verify-project' || r.name?.includes('verify'));
    expect(found, 'verify-project should be indexed after scan').toBeTruthy();

    // Cleanup
    await fetch(`${DAEMON_URL}/api/scan/roots/${rootId}`, { method: 'DELETE' });
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

  test('projects: POST creates a project (201), GET retrieves it, DELETE removes it', async () => {
    const name = `E2E Project ${RUN}`;

    // POST returns 201 CREATED
    const createResp = await fetch(`${DAEMON_URL}/api/projects`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, description: 'created by e2e test' }),
    });
    expect(createResp.status, 'POST /api/projects should return 201').toBe(201);
    const created = await createResp.json() as { ok: boolean; id: string };
    expect(created.ok).toBe(true);
    expect(created.id).toBeTruthy();

    // Verify it appears in GET /api/projects
    const list = await daemonGet<Project[]>('/api/projects');
    expect(list.find(p => String(p.id) === String(created.id))).toBeTruthy();

    // DELETE and confirm gone
    const del = await fetch(`${DAEMON_URL}/api/projects/${created.id}`, { method: 'DELETE' });
    expect(del.status).toBe(200);

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
      try { return JSON.parse(readFileSync(SETTINGS, 'utf8')); } catch { return null; }
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

    const hadHookBefore = hookEntries(readSettings(), 'SessionStart').includes(DEV_HOOK);

    // Configure
    const configureResp = await fetch(`${DAEMON_URL}/api/assistants/configure`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ acps: ['claude-code'] }),
    });
    expect(configureResp.status).toBe(200);
    await new Promise(r => setTimeout(r, 400));

    const after = readSettings();
    for (const ev of ['SessionStart', 'PreToolUse', 'PostToolUse', 'Stop']) {
      expect(
        hookEntries(after, ev).includes(DEV_HOOK),
        `${ev} hook should contain sensei-hook-dev.ts after configure`
      ).toBe(true);
    }

    // Remove (cleanup)
    await fetch(`${DAEMON_URL}/api/assistants/remove`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
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
});
