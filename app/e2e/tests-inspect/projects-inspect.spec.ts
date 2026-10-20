/**
 * #288 against the REAL daemon: the projects index and a project window render
 * the daemon's reads — each active card's FTR and the sidebar's match
 * `/api/projects/{id}/ftr` — with no error state and nothing "unavailable".
 * Run: `bun run test:inspect -- projects`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const pct = (r: number | null) => (r == null ? '—' : String(Math.round(r * 100)));

test('projects index and project window show the daemon reads', async ({ tauriPage }) => {
  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();

  // Grid (cards) or list (rows) — whichever view the saved preference shows.
  await navigateToScreen(
    tauriPage,
    '/projects',
    '[data-project-card], [data-project-row], [data-screen-state="error"]',
    20_000,
  );
  const cards = (await tauriPage.evaluate(`Array.from(document.querySelectorAll(
    '[data-project-card][data-status="active"], [data-project-row][data-status="active"]',
  )).map((c) => ({
    id: c.getAttribute('data-project-card') ?? c.getAttribute('data-project-row'),
    ftr: c.hasAttribute('data-project-card')
      ? c.querySelector('.grid .display')?.textContent?.trim() ?? null
      : (c.textContent.match(/(\\d+|—)%? ftr/) ?? [])[1] ?? null,
  }))`)) as { id: string; ftr: string | null }[];
  const bad = (await tauriPage.evaluate(
    `document.querySelectorAll('[data-screen-state="error"], [data-ftr-unavailable]').length`,
  )) as number;
  const expected = await Promise.all(
    cards.map(async (c) => pct((await (await fetch(`${DAEMON_URL}/api/projects/${c.id}/ftr`)).json()).ftr14d)),
  );
  console.log(`\n[inspect] ${projects.length} projects; active cards ${JSON.stringify(cards.map((c, i) => [c.ftr, expected[i]]))}`);
  expect(bad).toBe(0);
  expect(cards.length).toBeGreaterThan(0);
  expect(cards.map((c) => c.ftr)).toEqual(expected);

  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const ftr = (await (await fetch(`${DAEMON_URL}/api/projects/${sensei!.id}/ftr`)).json()).ftr14d;
  // The bridge's waitForSelector does not match a bare attribute; wait on the
  // health block, then read the value from the page.
  await navigateToScreen(tauriPage, `/project/${sensei!.id}/overview`, '[data-component="project-health"]', 20_000);
  const side = (await tauriPage.evaluate(`document.querySelector('[data-ftr-value]')?.textContent?.trim() ?? null`)) as string;
  console.log(`[inspect] sidebar FTR "${side}" (daemon ${ftr})`);
  expect(side).toBe(ftr == null ? '—' : `${pct(ftr)}%`);
});
