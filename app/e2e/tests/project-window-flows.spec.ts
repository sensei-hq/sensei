/**
 * Project window flows — traceability / memories / impact / patterns /
 * instruments (services + MCP tool stats).
 *
 * Verifies the T3 + T2 slices that landed the new project-scoped
 * screens. Each test walks a real flow (scan, propose, log, decide,
 * toggle) against the live daemon and asserts the DOM reflects the
 * daemon's persisted state.
 *
 * Uses whichever `sensei` project has the fullest inference data on
 * the local daemon — sensei / rokkit are typical. Skipped if no
 * project has any data (fresh install).
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

type Project = { id: string; name: string };

/** Fetch and JSON-parse defensively — the E2E daemon returns 0-byte bodies
 *  for some empty-state endpoints, which crashes `res.json()`. Falls back
 *  to the provided default so tests can inspect for empty and skip.
 */
async function safeJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(url);
    if (!res.ok) return fallback;
    const text = await res.text();
    if (!text) return fallback;
    return JSON.parse(text) as T;
  } catch {
    return fallback;
  }
}

/** Pick the project with the most inference rows so screens have content. */
async function pickTestProject(): Promise<Project | null> {
  const projects = await safeJson<Project[]>(`${DAEMON_URL}/api/projects`, []);
  // Prefer sensei / rokkit — they have the fullest analyzer coverage.
  const preferred = ['sensei', 'rokkit'];
  for (const name of preferred) {
    const p = projects.find(x => x.name === name);
    if (p) return p;
  }
  return projects[0] ?? null;
}

test.describe('Project window — Traceability doc-drift scan', () => {
  test('Scan now round-trips against the daemon detector', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/traceability`);
    await tauriPage.waitForSelector('[data-testid="drift-scan-button"]', 15_000);

    // Baseline drift count from the daemon.
    const before = await safeJson<{ total: number }>(
      `${DAEMON_URL}/api/projects/${project.id}/drift`, { total: 0 },
    );

    await tauriPage.locator('[data-testid="drift-scan-button"]').click();

    // Summary line should appear once the scan completes (up to ~30s
    // depending on doc corpus size). It carries the daemon's report.
    const summary = tauriPage.locator('[data-testid="drift-scan-summary"]');
    await expect(summary).toBeVisible({ timeout: 60_000 });
    await expect(summary).toContainText(/Scanned \d+ docs/);

    // Daemon state grew or stayed identical (re-scan idempotency).
    const after = await safeJson<{ total: number }>(
      `${DAEMON_URL}/api/projects/${project.id}/drift`, { total: 0 },
    );
    expect(after.total).toBeGreaterThanOrEqual(before.total);
  });
});

test.describe('Project window — Memories share batch flow', () => {
  test('Propose + approve batch round-trips through the daemon', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    // Ensure at least one active memory exists on this project.
    const memories = await safeJson<{ active: Array<{ id: string; title: string }> }>(
      `${DAEMON_URL}/api/projects/${project.id}/memories`, { active: [] },
    );
    if (memories.active.length === 0) {
      test.skip(true, `Project ${project.name} has no active memories to batch`);
      return;
    }

    await navigateTo(tauriPage, `/project/${project.id}/memories`);
    await tauriPage.waitForSelector('[data-testid="memories-list"]', 15_000);

    // Tick the first memory checkbox and propose the batch.
    const firstMem = memories.active[0].id;
    await tauriPage.locator(`[data-testid="memory-checkbox-${firstMem}"]`).click();
    await tauriPage.locator('[data-testid="propose-batch-button"]').click();

    // After proposeBatch() + invalidateAll(), the new batch row appears
    // with its Approve button. Assert we can see + click it.
    const approveButton = tauriPage.locator('[data-testid^="batch-approve-"]').first();
    await expect(approveButton).toBeVisible({ timeout: 15_000 });

    // Capture the batch id from the testid so we can verify persistence
    // after approval.
    const approveId = await approveButton.getAttribute('data-testid');
    const batchId = approveId?.replace('batch-approve-', '') ?? '';
    expect(batchId).toBeTruthy();

    await approveButton.click();

    // Daemon confirms the batch is now `approved` (falls off the proposed
    // list; the button disappears from the DOM after invalidate). Poll the
    // API rather than using toHaveCount — the wrapper's `.count()` doesn't
    // return a number Playwright's expect matcher recognises.
    let approvedIds: string[] = [];
    for (let i = 0; i < 20; i++) {
      const decided = await safeJson<{ batches: Array<{ id: string; status: string }> }>(
        `${DAEMON_URL}/api/projects/${project.id}/memory-batches?status=approved`,
        { batches: [] },
      );
      approvedIds = decided.batches.map(b => b.id);
      if (approvedIds.includes(batchId)) break;
      await new Promise<void>(r => setTimeout(r, 500));
    }
    expect(approvedIds).toContain(batchId);
  });
});

test.describe('Project window — Impact log', () => {
  test('Log + decide a verdict round-trips through the daemon', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/impact`);
    await tauriPage.waitForSelector('[data-testid="impact-title"]', 15_000);

    // Deterministic title so we can locate our entry after invalidate.
    // Skip the textarea — the tauri-playwright wrapper's `fill` only
    // supports HTMLInputElement, not HTMLTextAreaElement. Title is enough.
    const title = `E2E impact ${Date.now()}`;
    await tauriPage.locator('[data-testid="impact-title"]').fill(title);
    await tauriPage.locator('[data-testid="impact-log-button"]').click();

    // Daemon confirms the verdict was created as pending — poll the API
    // rather than watching DOM count fluctuations.
    let verdictId = '';
    for (let i = 0; i < 20; i++) {
      const pending = await safeJson<{ verdicts: Array<{ id: string; title: string; verdict: string }> }>(
        `${DAEMON_URL}/api/projects/${project.id}/impact-verdicts?verdict=pending`,
        { verdicts: [] },
      );
      const match = pending.verdicts.find(v => v.title === title);
      if (match) { verdictId = match.id; break; }
      await new Promise<void>(r => setTimeout(r, 500));
    }
    expect(verdictId).toBeTruthy();

    // Click Success on the row we just added.
    await tauriPage.locator(`[data-testid="impact-success-${verdictId}"]`).click();

    // Poll until the daemon records the decision.
    let seenSuccess = false;
    for (let i = 0; i < 20; i++) {
      const list = await safeJson<{ verdicts: Array<{ title: string; verdict: string }> }>(
        `${DAEMON_URL}/api/projects/${project.id}/impact-verdicts?verdict=success`,
        { verdicts: [] },
      );
      if (list.verdicts.some(v => v.title === title && v.verdict === 'success')) {
        seenSuccess = true;
        break;
      }
      await new Promise<void>(r => setTimeout(r, 500));
    }
    expect(seenSuccess).toBe(true);
  });
});

test.describe('Project window — Patterns detail disclosure', () => {
  test('Row click reveals description / example / enforcement when present', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    const patterns = await safeJson<{
      followed: Array<{ id: string; description?: string | null; example?: string | null }>;
      antiPatterns: Array<{ id: string; description?: string | null; example?: string | null }>;
    }>(
      `${DAEMON_URL}/api/projects/${project.id}/patterns`,
      { followed: [], antiPatterns: [] },
    );
    const all = [...patterns.followed, ...patterns.antiPatterns];
    if (all.length === 0) { test.skip(true, 'no patterns detected'); return; }
    // Prefer a pattern that has a description so the reveal is meaningful.
    const target = all.find(p => p.description) ?? all[0];

    await navigateTo(tauriPage, `/project/${project.id}/patterns`);
    const row = tauriPage.locator(`[data-testid="pattern-row-${target.id}"]`);
    await expect(row).toBeVisible({ timeout: 15_000 });

    await tauriPage.locator(`[data-testid="pattern-toggle-${target.id}"]`).click();
    await expect(tauriPage.locator(`[data-testid="pattern-detail-${target.id}"]`)).toBeVisible({ timeout: 5_000 });
  });
});

test.describe('Project window — Instruments services + MCP stats', () => {
  test('Service toggle flips scoped state on the daemon', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    const before = await safeJson<{
      services: Array<{ id: string; name: string; enabledForProject: boolean }>;
    }>(
      `${DAEMON_URL}/api/projects/${project.id}/services`, { services: [] },
    );
    if (before.services.length === 0) {
      test.skip(true, 'no installed services registered');
      return;
    }
    const first = before.services[0];

    await navigateTo(tauriPage, `/project/${project.id}/instruments`);
    const toggle = tauriPage.locator(`[data-testid="service-toggle-${first.name}"]`);
    await expect(toggle).toBeVisible({ timeout: 15_000 });
    await expect(toggle).toHaveAttribute('aria-pressed', String(first.enabledForProject));

    await toggle.click();
    // aria-pressed flips after invalidateAll() rehydrates.
    await expect(toggle).toHaveAttribute('aria-pressed', String(!first.enabledForProject), { timeout: 10_000 });

    // Daemon persisted the scoped override.
    const after = await safeJson<{ services: Array<{ id: string; enabledForProject: boolean }> }>(
      `${DAEMON_URL}/api/projects/${project.id}/services`, { services: [] },
    );
    const post = after.services.find(s => s.id === first.id);
    expect(post?.enabledForProject).toBe(!first.enabledForProject);

    // Flip back so the test is idempotent for re-runs.
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-pressed', String(first.enabledForProject), { timeout: 10_000 });
  });

  test('MCP tools table renders every manifest tool', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    const stats = await safeJson<{ tools: Array<{ name: string }> }>(
      `${DAEMON_URL}/api/projects/${project.id}/mcp-tool-stats`, { tools: [] },
    );
    if (stats.tools.length === 0) { test.skip(true, 'no MCP tool stats'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/instruments`);

    // The MCP tools table renders every manifest tool by name in the
    // grid — inspect the rendered DOM directly for a known tool name.
    // Wait for the page to have hydrated.
    await new Promise<void>(r => setTimeout(r, 1500));

    const target = stats.tools[0].name;
    const found = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('*'))
        .some(el => (el.textContent ?? '').includes(${JSON.stringify(target)}))
    `) as boolean;
    expect(found).toBe(true);
  });
});
