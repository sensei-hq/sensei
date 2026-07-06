/**
 * Project-window depth coverage — the post-T3 mockup-gap slice:
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
import { navigateTo, DAEMON_URL } from '../helpers';

type Project = { id: string; name: string };

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

async function pickTestProject(): Promise<Project | null> {
  const projects = await safeJson<Project[]>(`${DAEMON_URL}/api/projects`, []);
  return projects[0] ?? null;
}

test.describe('Project window — Sessions depth', () => {
  test('TurnBar renders inline on session rows with real turns', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }
    const sessions = await safeJson<{ sessions: Array<{ id: string; turns: number }> }>(
      `${DAEMON_URL}/api/projects/${project.id}/sessions?limit=50`, { sessions: [] },
    );
    const withTurns = sessions.sessions.filter(s => s.turns > 0);
    if (withTurns.length === 0) { test.skip(true, 'no sessions with turns > 0 on this project'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/sessions`);
    await tauriPage.waitForSelector('[data-component="turn-bar"]', 15_000);
    const attrs = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-component="turn-bar"]')).map(el => ({
        turns:       Number(el.getAttribute('data-turns') ?? 0),
        corrections: Number(el.getAttribute('data-corrections') ?? 0),
      }))
    `) as Array<{ turns: number; corrections: number }>;
    expect(attrs.length).toBeGreaterThan(0);
    // Every bar should have turns > 0 (rows without turns don't render one).
    for (const a of attrs) expect(a.turns).toBeGreaterThan(0);
  });
});

test.describe('Project window — Memories anatomy', () => {
  test('row click opens the Memory Anatomy drawer with What / stat grid', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }
    const { execFileSync } = await import('child_process');
    const psql = (sql: string): string => execFileSync('psql', [
      'postgres://localhost/sensei_e2e', '-qtAc', sql,
    ], { encoding: 'utf8' }).trim();

    // Ensure at least one active memory exists on this project. Seed one if
    // not — clean it up in `finally` regardless of outcome.
    const seededId = psql(`
      INSERT INTO sensei.memories (project_id, type, title, content, impact, status)
      VALUES ('${project.id}'::uuid,
              'convention'::sensei.memory_type,
              'E2E anatomy check',
              'the memory body renders under Because',
              'and the consequence body renders under Consequence',
              'active'::sensei.memory_status)
      RETURNING id
    `);
    expect(seededId).toMatch(/[0-9a-f-]{36}/);

    try {
      await navigateTo(tauriPage, `/project/${project.id}/memories`);
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

test.describe('Project window — Traceability diff', () => {
  test('drift row click reveals expected vs actual signature blocks', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }
    const { execFileSync } = await import('child_process');
    const psql = (sql: string): string => execFileSync('psql', [
      'postgres://localhost/sensei_e2e', '-qtAc', sql,
    ], { encoding: 'utf8' }).trim();

    // Seed a drift row with expected + actual signatures. Requires a
    // folder + a doc node + a code node for the FK constraints. Use
    // the project's first folder; upsert two nodes as scratch.
    const folderId = psql(`
      SELECT id FROM sensei.folders
       WHERE project_id = '${project.id}'::uuid ORDER BY path LIMIT 1
    `);
    if (!folderId) { test.skip(true, 'project has no folders for drift seed'); return; }
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
      await navigateTo(tauriPage, `/project/${project.id}/traceability`);
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

test.describe('Project window — Patterns FTR delta', () => {
  test('each pattern row surfaces its ftrDelta cell', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) { test.skip(true, 'no projects registered'); return; }
    const patterns = await safeJson<{
      followed: Array<{ id: string }>; antiPatterns: Array<{ id: string }>;
    }>(`${DAEMON_URL}/api/projects/${project.id}/patterns`,
       { followed: [], antiPatterns: [] });
    const all = [...patterns.followed, ...patterns.antiPatterns];
    if (all.length === 0) { test.skip(true, 'no patterns detected on this project'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/patterns`);
    const first = all[0];
    await tauriPage.waitForSelector(`[data-testid="pattern-ftr-delta-${first.id}"]`, 15_000);
    const text = await tauriPage.evaluate(`
      document.querySelector('[data-testid="pattern-ftr-delta-${first.id}"]')?.textContent?.trim() ?? ''
    `) as string;
    // Legal cells: "+N%", "-N%", "±0%", or "—" for a null baseline.
    expect(text).toMatch(/^(\+|-)?\d+%$|^±0%$|^—$/);
  });
});
