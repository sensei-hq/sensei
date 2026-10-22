/**
 * #288 against the REAL daemon: the workspaces index and a workspace window render
 * the daemon's reads — each active card's FTR and the sidebar's match
 * `/api/workspaces/{id}/ftr` — with no error state and nothing "unavailable".
 * Run: `bun run test:inspect -- workspaces`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const pct = (r: number | null) => (r == null ? '—' : String(Math.round(r * 100)));

test('workspaces index and workspace window show the daemon reads', async ({ tauriPage }) => {
  const workspaces: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();

  // Grid (cards) or list (rows) — whichever view the saved preference shows.
  await navigateToScreen(
    tauriPage,
    '/workspaces',
    '[data-workspace-card], [data-workspace-row], [data-screen-state="error"]',
    20_000,
  );
  const cards = (await tauriPage.evaluate(`Array.from(document.querySelectorAll(
    '[data-workspace-card][data-status="active"], [data-workspace-row][data-status="active"]',
  )).map((c) => ({
    id: c.getAttribute('data-workspace-card') ?? c.getAttribute('data-workspace-row'),
    ftr: c.hasAttribute('data-workspace-card')
      ? c.querySelector('.grid .display')?.textContent?.trim() ?? null
      : (c.textContent.match(/(\\d+|—)%? ftr/) ?? [])[1] ?? null,
  }))`)) as { id: string; ftr: string | null }[];
  const bad = (await tauriPage.evaluate(
    `document.querySelectorAll('[data-screen-state="error"], [data-ftr-unavailable]').length`,
  )) as number;
  const expected = await Promise.all(
    cards.map(async (c) => pct((await (await fetch(`${DAEMON_URL}/api/workspaces/${c.id}/ftr`)).json()).ftr14d)),
  );
  console.log(`\n[inspect] ${workspaces.length} workspaces; active cards ${JSON.stringify(cards.map((c, i) => [c.ftr, expected[i]]))}`);
  expect(bad).toBe(0);
  expect(cards.length).toBeGreaterThan(0);
  expect(cards.map((c) => c.ftr)).toEqual(expected);

  const sensei = workspaces.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei workspace on this daemon');
  const ftr = (await (await fetch(`${DAEMON_URL}/api/workspaces/${sensei!.id}/ftr`)).json()).ftr14d;
  // The bridge's waitForSelector does not match a bare attribute; wait on the
  // health block, then read the value from the page.
  await navigateToScreen(tauriPage, `/workspace/${sensei!.id}/overview`, '[data-component="workspace-health"]', 20_000);
  const side = (await tauriPage.evaluate(`document.querySelector('[data-ftr-value]')?.textContent?.trim() ?? null`)) as string;
  console.log(`[inspect] sidebar FTR "${side}" (daemon ${ftr})`);
  expect(side).toBe(ftr == null ? '—' : `${pct(ftr)}%`);
});
