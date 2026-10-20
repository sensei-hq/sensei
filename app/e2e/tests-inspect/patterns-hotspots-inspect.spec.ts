/**
 * #287 against the REAL daemon: this repository's Patterns screen shows the
 * daemon's anti-patterns — rework folded into one "rework hotspots" row whose
 * opened detail lists the daemon's directory groups. Read from the DOM: the
 * sign-in overlay (#283) is modal while it is up.
 * Run: `bun run test:inspect -- patterns-hotspots`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

test('patterns fold rework into hotspots', async ({ tauriPage }) => {
  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const { antiPatterns } = await (await fetch(`${DAEMON_URL}/api/projects/${sensei!.id}/patterns`)).json();
  const hot = antiPatterns.find((p: { id: string }) => p.id === 'rework-hotspots');
  const dirs = hot.groups.map((g: { directory: string }) => `pattern-group-${g.directory}`);

  await navigateTo(tauriPage, `/project/${sensei!.id}/patterns`);
  await expect
    .poll(() => tauriPage.evaluate(`!!document.querySelector('[data-testid="pattern-toggle-rework-hotspots"]')`), { timeout: 30_000 })
    .toBe(true);
  await tauriPage.evaluate(`document.querySelector('[data-testid="pattern-toggle-rework-hotspots"]').click()`);
  await new Promise((r) => setTimeout(r, 800));
  const seen = (await tauriPage.evaluate(`({
    rows: document.querySelectorAll('[data-testid^="pattern-row-"]').length,
    groups: [...document.querySelectorAll('[data-testid^="pattern-group-"]')].map((g) => g.getAttribute('data-testid')),
  })`)) as { rows: number; groups: string[] };
  console.log(`\n[inspect] patterns rows ${seen.rows}; hotspots ${hot.instanceCount} files in ${seen.groups.length} groups`);
  expect(seen.groups).toEqual(dirs);
});
