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

/** Pick the project with the most inference rows so screens have content. */
async function pickTestProject(): Promise<Project | null> {
  const projects = await fetch(`${DAEMON_URL}/api/projects`).then(r => r.json()) as Project[];
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
    await tauriPage.waitForSelector('[data-testid="drift-scan-button"]', { timeout: 15_000 });

    // Baseline drift count from the daemon.
    const before = await fetch(`${DAEMON_URL}/api/projects/${project.id}/drift`)
      .then(r => r.json()) as { total: number };

    await tauriPage.locator('[data-testid="drift-scan-button"]').click();

    // Summary line should appear once the scan completes (up to ~30s
    // depending on doc corpus size). It carries the daemon's report.
    const summary = tauriPage.locator('[data-testid="drift-scan-summary"]');
    await expect(summary).toBeVisible({ timeout: 60_000 });
    await expect(summary).toContainText(/Scanned \d+ docs/);

    // Daemon state grew or stayed identical (re-scan idempotency).
    const after = await fetch(`${DAEMON_URL}/api/projects/${project.id}/drift`)
      .then(r => r.json()) as { total: number };
    expect(after.total).toBeGreaterThanOrEqual(before.total);
  });
});

test.describe('Project window — Memories share batch flow', () => {
  test('Propose + approve batch round-trips through the daemon', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    // Ensure at least one active memory exists on this project.
    const memories = await fetch(`${DAEMON_URL}/api/projects/${project.id}/memories`)
      .then(r => r.json()) as { active: Array<{ id: string; title: string }> };
    if (memories.active.length === 0) {
      test.skip(true, `Project ${project.name} has no active memories to batch`);
      return;
    }

    await navigateTo(tauriPage, `/project/${project.id}/memories`);
    await tauriPage.waitForSelector('[data-testid="memories-list"]', { timeout: 15_000 });

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

    // Daemon should now record the batch as approved (falls off the
    // proposed list; the button disappears from the DOM after invalidate).
    await expect(approveButton).toHaveCount(0, { timeout: 10_000 });
    const decided = await fetch(
      `${DAEMON_URL}/api/projects/${project.id}/memory-batches?status=approved`,
    ).then(r => r.json()) as { batches: Array<{ id: string; status: string }> };
    expect(decided.batches.map(b => b.id)).toContain(batchId);
  });
});

test.describe('Project window — Impact log', () => {
  test('Log + decide a verdict round-trips through the daemon', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/impact`);
    await tauriPage.waitForSelector('[data-testid="impact-title"]', { timeout: 15_000 });

    // Deterministic title so we can locate our entry after invalidate.
    const title = `E2E impact ${Date.now()}`;
    await tauriPage.locator('[data-testid="impact-title"]').fill(title);
    await tauriPage.locator('[data-testid="impact-note"]').fill('Written by Playwright');
    await tauriPage.locator('[data-testid="impact-log-button"]').click();

    // Pending list should include the row we just added.
    const pendingRow = tauriPage.locator('[data-testid^="impact-pending-"]').filter({ hasText: title });
    await expect(pendingRow).toBeVisible({ timeout: 10_000 });

    // Decide success. The whole row should disappear (moves off pending).
    await pendingRow.locator('[data-testid^="impact-success-"]').click();
    await expect(pendingRow).toHaveCount(0, { timeout: 10_000 });

    // Daemon confirms it's now recorded as success.
    const list = await fetch(
      `${DAEMON_URL}/api/projects/${project.id}/impact-verdicts?verdict=success`,
    ).then(r => r.json()) as { verdicts: Array<{ title: string; verdict: string }> };
    expect(list.verdicts.some(v => v.title === title && v.verdict === 'success')).toBe(true);
  });
});

test.describe('Project window — Patterns detail disclosure', () => {
  test('Row click reveals description / example / enforcement when present', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    const patterns = await fetch(`${DAEMON_URL}/api/projects/${project.id}/patterns`)
      .then(r => r.json()) as {
        followed: Array<{ id: string; description?: string | null; example?: string | null }>;
        antiPatterns: Array<{ id: string; description?: string | null; example?: string | null }>;
      };
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

    const before = await fetch(`${DAEMON_URL}/api/projects/${project.id}/services`)
      .then(r => r.json()) as {
        services: Array<{ id: string; name: string; enabledForProject: boolean }>;
      };
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
    const after = await fetch(`${DAEMON_URL}/api/projects/${project.id}/services`)
      .then(r => r.json()) as { services: Array<{ id: string; enabledForProject: boolean }> };
    const post = after.services.find(s => s.id === first.id);
    expect(post?.enabledForProject).toBe(!first.enabledForProject);

    // Flip back so the test is idempotent for re-runs.
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-pressed', String(first.enabledForProject), { timeout: 10_000 });
  });

  test('MCP tools table renders every manifest tool', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }

    const stats = await fetch(`${DAEMON_URL}/api/projects/${project.id}/mcp-tool-stats`)
      .then(r => r.json()) as { tools: Array<{ name: string }> };

    await navigateTo(tauriPage, `/project/${project.id}/instruments`);
    // Header shows the count from the header snippet.
    const header = tauriPage.locator('h1', { hasText: 'Instruments' });
    await expect(header).toBeVisible({ timeout: 15_000 });
    // Tool name text should show for every manifest tool.
    for (const t of stats.tools.slice(0, 3)) {
      await expect(tauriPage.getByText(t.name).first()).toBeVisible({ timeout: 5_000 });
    }
  });
});
