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
  // The budget line sits under the header's numbers, in a band a reader may have closed.
  await tauriPage.evaluate(`(() => {
    const t = document.querySelector('[data-testid="diagram-stats-toggle"]');
    if (t?.getAttribute('aria-pressed') === 'false') t.click();
  })()`);
  await expect
    .poll(() => tauriPage.evaluate(`!!document.querySelector('[data-testid="diagram-budget"]')`), { timeout: 10_000 })
    .toBe(true);
  const budget = (await tauriPage.evaluate(`document.querySelector('[data-testid="diagram-budget"]')?.textContent ?? ''`)) as string;
  expect(budget).toContain(`drawing ${l0.budget.drawn} of ${l0.budget.modules} modules`);
  // The daemon now counts each block's files by role, and the referenced libraries.
  const roleSum = (b: { roles: Record<string, number> }) => Object.values(b.roles).reduce((a, v) => a + v, 0);
  expect(roleSum(biggest)).toBe(biggest.files);
  // With nothing selected the side panel says what to act on (open it if closed).
  await tauriPage.evaluate(`(() => {
    const t = document.querySelector('[data-testid="diagram-side-toggle"]');
    if (t?.getAttribute('aria-pressed') === 'false') t.click();
  })()`);
  await expect
    .poll(() => tauriPage.evaluate(`!!document.querySelector('[data-testid="codemap-takeaways"]')`), { timeout: 10_000 })
    .toBe(true);
  const about = (await tauriPage.evaluate(`!!document.querySelector('[data-testid="diagram-about-toggle"]')`)) as boolean;
  const libraries = (await tauriPage.evaluate(`document.querySelector('[data-testid="codemap-libraries"]')?.textContent?.trim() ?? ''`)) as string;
  expect(libraries).toBe(l0.libraries.count ? `${l0.libraries.count} referenced libraries · not members` : '');

  // One click opens a module (M17); the inspector names it.
  await tauriPage.evaluate(`document.querySelector('[data-testid="block-${biggest.id}"]')?.dispatchEvent(new MouseEvent('click', { bubbles: true }))`);
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

  console.log(`\n[inspect] codemap ${ws.name}: ${l0.blocks.length} blocks, ${l0.bands.length} bands, ${l0.libraries.count} libraries; opened ${biggest.id} → ${file.slice(5)}; crumbs ${crumbs}; back ${back}; about ${about}; errors ${errors}`);
  expect([back, errors, about]).toEqual([true, 0, true]);
});

// The band filters against the daemon's own L0: "Violations & cycles" draws exactly
// the upward, skipping and in-cycle bands, derived here independently of the app.
test('violations & cycles draws exactly the violating bands', async ({ tauriPage }) => {
  const workspaces = await json('/api/workspaces');
  const rows = Array.isArray(workspaces) ? workspaces : workspaces.workspaces;
  const ws = rows.find((w: { name: string }) => w.name === 'sensei') ?? rows[0];
  const l0 = await json(`/api/workspaces/${ws.id}/diagrams/codemap?groupBy=layers`);
  const inCycle = new Map<string, number>();
  (l0.cycles as { members?: string[] }[]).forEach((c, i) => (c.members ?? []).forEach((m) => inCycle.set(m, i)));
  const hub = new Set(l0.blocks.filter((b: { hub: boolean }) => b.hub).map((b: { id: string }) => b.id));
  const expected = l0.bands.filter((b: { source: string; target: string; direction: string | null }) =>
    !hub.has(b.source) && !hub.has(b.target) &&
    (b.direction === 'up' || b.direction === 'skip' || (inCycle.has(b.source) && inCycle.get(b.source) === inCycle.get(b.target)))).length;

  await navigateTo(tauriPage, `/workspace/${ws.id}/diagrams/structure`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('g[data-testid^="block-"]').length`), { timeout: 60_000 })
    .toBeGreaterThan(0);
  const all = (await tauriPage.evaluate(`document.querySelectorAll('path[data-testid^="band-"]').length`)) as number;
  await tauriPage.evaluate(`document.querySelector('[data-testid="codemap-edges"] [aria-label="Violations & cycles"]')?.click()`);
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('path[data-testid^="band-"]').length`), { timeout: 10_000 })
    .toBe(expected);
  console.log(`\n[inspect] codemap filter ${ws.name}: ${all} bands → ${expected} violations & cycles`);
});

// Every lens, opened from its own route, draws on the same canvas without an error,
// against the daemon's data.
test('each lens draws against the real daemon', async ({ tauriPage }) => {
  test.setTimeout(240_000);
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
    ['matrix', '[data-testid="codemap-matrix"] td[data-cell]'],
    ['zones', '[data-testid^="zone-"]'],
    ['coupling', '[data-testid^="band-"]'],
  ] as const) {
    const started = Date.now();
    await navigateTo(tauriPage, `/workspace/${ws.id}/diagrams/${lens}`);
    await expect
      .poll(() => tauriPage.evaluate(`document.querySelectorAll('${sel}').length`), { timeout: 60_000, message: `${lens} draws` })
      .toBeGreaterThan(0);
    seen[lens] = (await tauriPage.evaluate(`document.querySelectorAll('${sel}').length`)) as number;
    console.log(`[inspect] lens ${lens}: ${seen[lens]} drawn in ${Date.now() - started} ms`);
  }
  const summary = (await tauriPage.evaluate(`document.querySelector('[data-testid="coupling-summary"]')?.textContent ?? ''`)) as string;
  const errors = (await tauriPage.evaluate(`document.querySelectorAll('[data-testid="codemap-error"]').length`)) as number;
  console.log(`\n[inspect] codemap lenses ${JSON.stringify(seen)}; coupling: ${summary.trim()}; errors ${errors}`);
  expect(summary).toContain(`from ${cochange.commits} commits`);
  expect(errors).toBe(0);
});
