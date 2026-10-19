/**
 * Inspect Settings · Projects against the REAL daemon (#271). Read-only: it
 * changes nothing. It checks that each project lists only its repositories
 * (git checkouts and subtrees, never workspace modules or plain folders), that
 * projects with no repository here are collapsed, and that search narrows the
 * list. Captures /tmp/sensei-shots/settings-projects*.png. Run: `bun run test:inspect`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const REPOSITORY_KINDS = new Set(['git', 'subtree']);

type Folder = { kind: string };
type Project = { id: string; name: string; folders?: Folder[] };

test('settings · projects lists repositories, not folders', async ({ tauriPage }) => {
  mkdirSync(DIR, { recursive: true });
  const projects: Project[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const expected = new Map(
    projects.map((p) => [p.id, (p.folders ?? []).filter((f) => REPOSITORY_KINDS.has(f.kind)).length]),
  );
  const placedIds = [...expected].filter(([, n]) => n > 0).map(([id]) => id);

  await navigateToScreen(tauriPage, '/settings/projects', '[data-testid="settings-projects"]');
  await expect(tauriPage.locator('[data-placed]')).toBeVisible({ timeout: 20_000 });
  // An expired identity opens the sign-in overlay over the screen; close it so
  // the screenshots show the screen under inspection.
  await tauriPage.evaluate(
    `Array.from(document.querySelectorAll('[role="dialog"] button')).find((b) => b.textContent.trim() === 'Close')?.click()`,
  );
  await new Promise((r) => setTimeout(r, 300));

  const shown = (await tauriPage.evaluate(`
    (() => Array.from(document.querySelectorAll('[data-placed] [data-testid^="project-card-"]')).map((c) => ({
      id: c.getAttribute('data-testid').replace('project-card-', ''),
      count: c.querySelector('[data-repo-count]')?.textContent?.trim() ?? '',
      repos: c.querySelectorAll('[data-repo]').length,
    })))()
  `)) as Array<{ id: string; count: string; repos: number }>;
  const unplaced = (await tauriPage.evaluate(
    `document.querySelector('[data-unplaced] summary')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null`,
  )) as string | null;
  console.log(`\n[inspect] settings/projects — placed: ${shown.length}, unplaced: ${unplaced}`);
  for (const s of shown) console.log(`[inspect]   ${s.id}: ${s.count} (${s.repos} repository rows)`);
  await tauriPage.screenshot({ path: `${DIR}/settings-projects.png` });

  expect(shown.map((s) => s.id).sort()).toEqual([...placedIds].sort());
  for (const s of shown) {
    expect(s.repos, `project ${s.id} lists only its repositories`).toBe(expected.get(s.id));
  }
  if (projects.length > placedIds.length) {
    expect(unplaced).toContain(String(projects.length - placedIds.length));
  }

  // Search narrows to a project by name.
  const first = projects.find((p) => p.id === placedIds[0]);
  if (first) {
    await tauriPage.locator('input[type="search"]').fill(first.name);
    await new Promise((r) => setTimeout(r, 500));
    const after = (await tauriPage.evaluate(
      `document.querySelectorAll('[data-placed] [data-testid^="project-card-"]').length`,
    )) as number;
    console.log(`[inspect]   search "${first.name}" → ${after} card(s)`);
    await tauriPage.screenshot({ path: `${DIR}/settings-projects-search.png` });
    expect(after).toBeGreaterThanOrEqual(1);
    expect(after).toBeLessThanOrEqual(placedIds.length);
  }
});
