/**
 * #85 against the REAL daemon: this repository's project Overview shows its
 * hotspots (the five most-edited files the daemon reports) and its patterns in
 * use (the counts the daemon reports). Run: `bun run test:inspect -- overview`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

test('overview shows hotspots and patterns in use', async ({ tauriPage }) => {
  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const id = sensei!.id;
  const { hotspots } = await (await fetch(`${DAEMON_URL}/api/projects/${id}/hotspots?days=7`)).json();
  const pats = await (await fetch(`${DAEMON_URL}/api/projects/${id}/patterns`)).json();

  await navigateToScreen(tauriPage, `/project/${id}/overview`, '[data-testid="overview-patterns"]', 20_000);
  const shown = (await tauriPage.evaluate(
    `Array.from(document.querySelectorAll('[data-hotspot]')).map((e) => e.getAttribute('data-hotspot'))`,
  )) as string[];
  const counts = (await tauriPage.evaluate(
    `document.querySelector('[data-pattern-counts]')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null`,
  )) as string | null;
  const errors = (await tauriPage.evaluate(`document.querySelectorAll('[data-overview-error]').length`)) as number;
  console.log(`\n[inspect] hotspots: ${shown.length} shown of ${hotspots.length} — ${shown.join(', ')}`);
  console.log(`[inspect] patterns: "${counts}" (daemon: ${pats.followed.length} followed, ${pats.antiPatterns.length} anti)`);
  expect(errors).toBe(0);
  expect(shown).toHaveLength(Math.min(5, hotspots.length));
  expect(counts).toBe(`${pats.followed.length} followed · ${pats.antiPatterns.length} anti-patterns`);
});
