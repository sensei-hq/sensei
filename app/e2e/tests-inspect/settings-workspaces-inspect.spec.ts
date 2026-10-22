/**
 * Inspect Settings · Workspaces against the REAL daemon (#271). Read-only: it
 * changes nothing. It checks that each workspace lists only its repositories
 * (git checkouts and subtrees, never workspace modules or plain folders), that
 * workspaces with no repository here are collapsed, and that search narrows the
 * list. Captures /tmp/sensei-shots/settings-workspaces*.png. Run: `bun run test:inspect`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen, shot } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const REPOSITORY_KINDS = new Set(['git', 'subtree']);

type Folder = { id: string; name: string; kind: string; abs_path: string; role: string | null };
type Workspace = { id: string; name: string; folders?: Folder[] };

test('settings · workspaces lists repositories, not folders', async ({ tauriPage }) => {
  mkdirSync(DIR, { recursive: true });
  const workspaces: Workspace[] = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const expected = new Map(
    workspaces.map((p) => [p.id, (p.folders ?? []).filter((f) => REPOSITORY_KINDS.has(f.kind)).length]),
  );
  const placedIds = [...expected].filter(([, n]) => n > 0).map(([id]) => id);

  await navigateToScreen(tauriPage, '/settings/workspaces', '[data-testid="settings-workspaces"]');
  await expect(tauriPage.locator('[data-placed]')).toBeVisible({ timeout: 20_000 });
  // An expired identity opens the sign-in overlay over the screen; close it so
  // the screenshots show the screen under inspection.
  await tauriPage.evaluate(
    `Array.from(document.querySelectorAll('[role="dialog"] button')).find((b) => b.textContent.trim() === 'Close')?.click()`,
  );
  await new Promise((r) => setTimeout(r, 300));

  const shown = (await tauriPage.evaluate(`
    (() => Array.from(document.querySelectorAll('[data-placed] [data-testid^="workspace-card-"]')).map((c) => ({
      id: c.getAttribute('data-testid').replace('workspace-card-', ''),
      count: c.querySelector('[data-repo-count]')?.textContent?.trim() ?? '',
      repos: c.querySelectorAll('[data-repo]').length,
    })))()
  `)) as Array<{ id: string; count: string; repos: number }>;
  const unplaced = (await tauriPage.evaluate(
    `document.querySelector('[data-unplaced] summary')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null`,
  )) as string | null;
  console.log(`\n[inspect] settings/workspaces — placed: ${shown.length}, unplaced: ${unplaced}`);
  for (const s of shown) console.log(`[inspect]   ${s.id}: ${s.count} (${s.repos} repository rows)`);
  await shot(tauriPage, `${DIR}/settings-workspaces.png`);

  expect(shown.map((s) => s.id).sort()).toEqual([...placedIds].sort());
  for (const s of shown) {
    expect(s.repos, `workspace ${s.id} lists only its repositories`).toBe(expected.get(s.id));
  }
  // A repository shows the roles the scan inferred for its packages (#271
  // "auto derive the category"). No checkout here nests another, so the
  // packages under its path are its own.
  const derived = (await tauriPage.evaluate(`
    Object.fromEntries(Array.from(document.querySelectorAll('[data-placed] [data-repo]')).map((r) => [
      r.closest('[data-testid^="workspace-card-"]').getAttribute('data-testid').replace('workspace-card-', '') + '/' + r.getAttribute('data-repo'),
      r.querySelector('[data-derived-roles]')?.getAttribute('data-derived-roles') ?? '',
    ]))
  `)) as Record<string, string>;
  for (const p of workspaces.filter((p) => placedIds.includes(p.id))) {
    for (const repo of (p.folders ?? []).filter((f) => REPOSITORY_KINDS.has(f.kind))) {
      const roles = new Set(
        (p.folders ?? [])
          .filter((f) => f.kind === 'module' && f.role && f.abs_path.startsWith(`${repo.abs_path}/`))
          .map((f) => f.role as string),
      );
      const key = `${p.id}/${repo.name}`;
      console.log(`[inspect]   ${key}: derived "${derived[key]}"`);
      expect(new Set((derived[key] ?? '').split(' ').filter(Boolean))).toEqual(roles);
    }
  }
  await tauriPage.evaluate(
    `document.querySelector('[data-derived-roles]')?.closest('[data-testid^="workspace-card-"]')?.scrollIntoView({ block: 'center' })`,
  );
  await shot(tauriPage, `${DIR}/settings-workspaces-derived.png`);

  if (workspaces.length > placedIds.length) {
    expect(unplaced).toContain(String(workspaces.length - placedIds.length));
  }

  // Search narrows to a workspace by name.
  const first = workspaces.find((p) => p.id === placedIds[0]);
  if (first) {
    await tauriPage.locator('input[type="search"]').fill(first.name);
    await new Promise((r) => setTimeout(r, 500));
    const after = (await tauriPage.evaluate(
      `document.querySelectorAll('[data-placed] [data-testid^="workspace-card-"]').length`,
    )) as number;
    console.log(`[inspect]   search "${first.name}" → ${after} card(s)`);
    await shot(tauriPage, `${DIR}/settings-workspaces-search.png`);
    expect(after).toBeGreaterThanOrEqual(1);
    expect(after).toBeLessThanOrEqual(placedIds.length);
  }
});
