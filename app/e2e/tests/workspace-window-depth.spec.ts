/**
 * Workspace-window depth coverage — the post-T3 mockup-gap slice:
 *   • Sessions: TurnBar visual + "Rework" column vocabulary.
 *   • Memories: What/Because/Consequence detail drawer opens on row click.
 *   • Traceability: expected-vs-actual diff toggle expands on click.
 *   • Patterns: FTR delta cell renders alongside each pattern row.
 *
 * Uses the sensei_e2e daemon's live data. When a specific fixture isn't
 * present (e.g. no memories on a fresh install), the test seeds via
 * psql — same pattern as the T3 Overview reject flow.
 */

import { test, expect } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';

type Workspace = { id: string; name: string };

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

async function pickTestWorkspace(): Promise<Workspace | null> {
  const workspaces = await safeJson<Workspace[]>(`${DAEMON_URL}/api/workspaces`, []);
  return workspaces[0] ?? null;
}

test.describe('Workspace window — Sessions depth', () => {
  test('session rows carry the shared quality-tone + corrections vocabulary', async ({ tauriPage }) => {
    const workspace = await pickTestWorkspace();
    if (!workspace) { test.skip(true, 'no workspaces registered'); return; }
    // Workspace Sessions now shares the Observatory digest rows; scope by the
    // widest window so the rendered 90d slice matches this fetch.
    // /api/sessions returns a digest object { stats, sessions: [...], … } — read
    // the nested array so the existence-guard skips honestly on a 0-session
    // workspace (a bare `.length` on the object is undefined → never skips).
    const scoped = await safeJson<{ sessions: Array<{ id: string }> }>(
      `${DAEMON_URL}/api/sessions?workspace=${workspace.id}&range=90d`, { sessions: [] },
    );
    if (scoped.sessions.length === 0) { test.skip(true, 'no sessions on this workspace'); return; }

    await navigateToScreen(tauriPage, `/workspace/${workspace.id}/sessions`, '[data-testid="workspace-sessions"]');
    await tauriPage.locator('[data-testid="range-90d"]').click();
    await tauriPage.waitForSelector('[data-session-row]', 15_000);

    // Every row carries a quality tone (data-quality) and the corrections
    // vocabulary every screen uses (lib/ftr correctionsView: "no corrections" / "N corrections" / "—").
    const rows = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-session-row]')).map(el => ({
        quality: el.getAttribute('data-quality') ?? '',
        text:    el.textContent ?? '',
      }))
    `) as Array<{ quality: string; text: string }>;
    expect(rows.length).toBeGreaterThan(0);
    for (const r of rows) {
      expect(['good', 'bad', 'ugly', 'neutral']).toContain(r.quality);
      expect(r.text).toMatch(/no corrections|\d+ corrections?|—/);
    }
  });
});

test.describe('Workspace window — Memories anatomy', () => {
  test('row click opens the Memory Anatomy drawer with What / stat grid', async ({ tauriPage }) => {
    const workspace = await pickTestWorkspace();
    if (!workspace) { test.skip(true, 'no workspaces registered'); return; }
    const { execFileSync } = await import('child_process');
    const psql = (sql: string): string => execFileSync('psql', [
      'postgres://localhost/sensei_e2e', '-qtAc', sql,
    ], { encoding: 'utf8' }).trim();

    // Ensure at least one active memory exists on this workspace. Seed one if
    // not — clean it up in `finally` regardless of outcome.
    const seededId = psql(`
      INSERT INTO sensei.memories (workspace_id, type, title, content, impact, status)
      VALUES ('${workspace.id}'::uuid,
              'convention'::sensei.memory_type,
              'E2E anatomy check',
              'the memory body renders under Because',
              'and the consequence body renders under Consequence',
              'active'::sensei.memory_status)
      RETURNING id
    `);
    expect(seededId).toMatch(/[0-9a-f-]{36}/);

    try {
      await navigateToScreen(tauriPage, `/workspace/${workspace.id}/memories`, '[data-component="workspace-main"][data-section="memories"]');
      await tauriPage.waitForSelector(`[data-testid="memory-open-${seededId}"]`, 15_000);
      await tauriPage.locator(`[data-testid="memory-open-${seededId}"]`).click();
      const anatomy = tauriPage.locator('[data-testid="memory-anatomy"]');
      await expect(anatomy).toBeVisible({ timeout: 5_000 });
      // The drawer must surface the three anatomy labels.
      const text = await tauriPage.evaluate(`
        document.querySelector('[data-testid="memory-anatomy"]')?.textContent ?? ''
      `) as string;
      expect(text).toContain('What');
      expect(text).toContain('Because');
      expect(text).toContain('Consequence');
      expect(text).toContain('E2E anatomy check');
    } finally {
      try { psql(`DELETE FROM sensei.memories WHERE id = '${seededId}'::uuid`); } catch {}
    }
  });
});

test.describe('Workspace window — Traceability diff', () => {
  test('drift row click reveals expected vs actual signature blocks', async ({ tauriPage }) => {
    const workspace = await pickTestWorkspace();
    if (!workspace) { test.skip(true, 'no workspaces registered'); return; }
    const { execFileSync } = await import('child_process');
    const psql = (sql: string): string => execFileSync('psql', [
      'postgres://localhost/sensei_e2e', '-qtAc', sql,
    ], { encoding: 'utf8' }).trim();

    // Seed a drift row with expected + actual signatures. Requires a
    // folder + a doc node + a code node for the FK constraints. Use
    // the workspace's first folder; upsert two nodes as scratch.
    // A workspace reaches its folders through its repositories (folders lost
    // workspace_id in 0.12); sensei.folder_workspaces is that join.
    const folderId = psql(`
      SELECT folder_id FROM sensei.folder_workspaces
       WHERE workspace_id = '${workspace.id}'::uuid ORDER BY folder_abs_path LIMIT 1
    `);
    if (!folderId) { test.skip(true, 'workspace has no folders for drift seed'); return; }
    const docNodeId = psql(`
      INSERT INTO sensei.nodes (folder_id, kind, name, file_path)
      VALUES ('${folderId}'::uuid, 'doc'::sensei.node_kind, '_test:doc', 'DOCS.md')
      RETURNING id
    `);
    const codeNodeId = psql(`
      INSERT INTO sensei.nodes (folder_id, kind, name, file_path)
      VALUES ('${folderId}'::uuid, 'function'::sensei.node_kind, 'foo', 'src/foo.ts')
      RETURNING id
    `);
    const driftId = psql(`
      INSERT INTO inference.drift_items (folder_id, doc_node_id, code_node_id, status, expected_signature, actual_signature, detail)
      VALUES ('${folderId}'::uuid, '${docNodeId}'::uuid, '${codeNodeId}'::uuid,
              'drifted'::sensei.drift_status,
              'fn foo(a: string): void',
              'fn foo(a: string, b: number): void',
              'seeded drift row')
      RETURNING id
    `);
    expect(driftId).toMatch(/[0-9a-f-]{36}/);

    try {
      await navigateToScreen(tauriPage, `/workspace/${workspace.id}/traceability`, '[data-component="workspace-main"][data-section="traceability"]');
      await tauriPage.waitForSelector(`[data-testid="drift-toggle-${driftId}"]`, 15_000);
      await tauriPage.locator(`[data-testid="drift-toggle-${driftId}"]`).click();
      const diff = tauriPage.locator(`[data-testid="drift-diff-${driftId}"]`);
      await expect(diff).toBeVisible({ timeout: 5_000 });
      const text = await tauriPage.evaluate(`
        document.querySelector('[data-testid="drift-diff-${driftId}"]')?.textContent ?? ''
      `) as string;
      expect(text).toContain('Expected');
      expect(text).toContain('Actual');
      expect(text).toContain('fn foo(a: string): void');
      expect(text).toContain('fn foo(a: string, b: number): void');
    } finally {
      try {
        psql(`DELETE FROM inference.drift_items WHERE id = '${driftId}'::uuid`);
        psql(`DELETE FROM sensei.nodes WHERE id IN ('${docNodeId}'::uuid, '${codeNodeId}'::uuid)`);
      } catch {}
    }
  });
});

test.describe('Workspace window — Patterns FTR delta', () => {
  test('each pattern row surfaces its ftrDelta cell', async ({ tauriPage }) => {
    const workspace = await pickTestWorkspace();
    if (!workspace) { test.skip(true, 'no workspaces registered'); return; }
    const patterns = await safeJson<{
      followed: Array<{ id: string }>; antiPatterns: Array<{ id: string }>;
    }>(`${DAEMON_URL}/api/workspaces/${workspace.id}/patterns`,
       { followed: [], antiPatterns: [] });
    const all = [...patterns.followed, ...patterns.antiPatterns];
    if (all.length === 0) { test.skip(true, 'no patterns detected on this workspace'); return; }

    await navigateToScreen(tauriPage, `/workspace/${workspace.id}/patterns`, '[data-component="workspace-main"][data-section="patterns"]');
    const first = all[0];
    await tauriPage.waitForSelector(`[data-testid="pattern-ftr-delta-${first.id}"]`, 15_000);
    const text = await tauriPage.evaluate(`
      document.querySelector('[data-testid="pattern-ftr-delta-${first.id}"]')?.textContent?.trim() ?? ''
    `) as string;
    // Legal cells: "+N%", "-N%", "±0%", or "—" for a null baseline.
    expect(text).toMatch(/^(\+|-)?\d+%$|^±0%$|^—$/);
  });
});
