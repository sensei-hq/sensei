/**
 * The Code map (M17) against the REAL daemon: a workspace's diagrams open on
 * modules, drawing what the daemon budgets; a module opens into its files by
 * role; a file opens into its symbols; the breadcrumb comes back up. Read from
 * the DOM. Run: `bun run test:inspect -- codemap`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();

test('the code map drills from modules to files to symbols', async ({ tauriPage }) => {
  const workspaces = await json('/api/workspaces');
  const rows = Array.isArray(workspaces) ? workspaces : workspaces.workspaces;
  const ws = rows.find((w: { name: string }) => w.name === 'sensei') ?? rows[0];
  const l0 = await json(`/api/workspaces/${ws.id}/diagrams/codemap?groupBy=layers`);
  const biggest = l0.blocks.filter((b: { folded: string[]; hub: boolean }) => !b.folded.length && !b.hub)
    .sort((a: { files: number }, b: { files: number }) => b.files - a.files)[0];

  await navigateTo(tauriPage, `/workspace/${ws.id}/diagrams/structure`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('g[data-testid^="block-"]').length`), { timeout: 60_000 })
    .toBe(l0.blocks.length);
  const budget = (await tauriPage.evaluate(`document.querySelector('[data-testid="codemap-budget"]')?.textContent ?? ''`)) as string;
  expect(budget).toContain(`drawing ${l0.budget.drawn} blocks for ${l0.budget.modules} modules`);

  await tauriPage.evaluate(`document.querySelector('[data-testid="block-${biggest.id}"]')?.dispatchEvent(new MouseEvent('click', { bubbles: true }))`);
  await tauriPage.evaluate(`document.querySelector('[data-testid="codemap-open"]')?.click()`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-testid^="column-"]').length`), { timeout: 30_000 })
    .toBeGreaterThan(0);
  const file = (await tauriPage.evaluate(`document.querySelector('[data-testid^="file-"]')?.getAttribute('data-testid') ?? ''`)) as string;
  await tauriPage.evaluate(`document.querySelector('[data-testid="${file}"]')?.click()`);
  await expect
    .poll(() => tauriPage.evaluate(`!!document.querySelector('[data-testid="codemap-symbols"]')`), { timeout: 30_000 })
    .toBe(true);
  const crumbs = (await tauriPage.evaluate(`[...document.querySelectorAll('[data-testid="codemap-crumbs"] button')].map((b) => b.textContent.trim()).join(' › ')`)) as string;
  await tauriPage.evaluate(`document.querySelector('[data-testid="codemap-crumbs"] button')?.click()`);
  const back = (await tauriPage.evaluate(`!!document.querySelector('[data-testid="codemap-modules"]')`)) as boolean;
  const errors = (await tauriPage.evaluate(`document.querySelectorAll('[data-testid="codemap-error"]').length`)) as number;

  console.log(`\n[inspect] codemap ${ws.name}: ${l0.blocks.length} blocks, ${l0.bands.length} bands; opened ${biggest.id} → ${file.slice(5)}; crumbs ${crumbs}; back ${back}; errors ${errors}`);
  expect([back, errors]).toEqual([true, 0]);
});

// Every lens, opened from its own route, draws on the same canvas without an error,
// against the daemon's data.
test('each lens draws against the real daemon', async ({ tauriPage }) => {
  const workspaces = await json('/api/workspaces');
  const rows = Array.isArray(workspaces) ? workspaces : workspaces.workspaces;
  const ws = rows.find((w: { name: string }) => w.name === 'sensei') ?? rows[0];
  const cochange = await json(`/api/workspaces/${ws.id}/diagrams/codemap/cochange`);

  // Structure first: the map's first read is the slow one.
  await navigateTo(tauriPage, `/workspace/${ws.id}/diagrams/structure`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('g[data-testid^="block-"]').length`), { timeout: 60_000 })
    .toBeGreaterThan(0);
  const seen: Record<string, number> = {};
  for (const [lens, sel] of [
    ['wheel', '[data-testid^="wheel-node-"]'],
    ['matrix', '[data-testid="codemap-matrix"] rect'],
    ['zones', '[data-testid^="zone-"]'],
    ['coupling', '[data-testid^="band-"]'],
  ] as const) {
    await navigateTo(tauriPage, `/workspace/${ws.id}/diagrams/${lens}`);
    await expect
      .poll(() => tauriPage.evaluate(`document.querySelectorAll('${sel}').length`), { timeout: 30_000, message: `${lens} draws` })
      .toBeGreaterThan(0);
    seen[lens] = (await tauriPage.evaluate(`document.querySelectorAll('${sel}').length`)) as number;
  }
  const summary = (await tauriPage.evaluate(`document.querySelector('[data-testid="coupling-summary"]')?.textContent ?? ''`)) as string;
  const errors = (await tauriPage.evaluate(`document.querySelectorAll('[data-testid="codemap-error"]').length`)) as number;
  console.log(`\n[inspect] codemap lenses ${JSON.stringify(seen)}; coupling: ${summary.trim()}; errors ${errors}`);
  expect(summary).toContain(`from ${cochange.commits} commits`);
  expect(errors).toBe(0);
});
