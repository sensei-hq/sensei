/**
 * #288 against the REAL daemon: Observatory Sessions and this repository's
 * project Sessions list the daemon's 7-day sessions, with no error state.
 * Run: `bun run test:inspect -- sessions-inspect`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const rows = `({
  error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
  rows: document.querySelectorAll('[data-testid="session-list"] > *').length,
})`;

test('sessions screens list the daemon sessions', async ({ tauriPage }) => {
  const all = (await (await fetch(`${DAEMON_URL}/api/sessions?range=7d`)).json()).sessions.length;
  await navigateToScreen(tauriPage, '/sessions', '[data-testid="range-chips"]', 60_000);
  await tauriPage.waitForSelector('[data-testid="session-list"], [data-screen-state="error"]', 20_000);
  const o = (await tauriPage.evaluate(rows)) as { error: string | null; rows: number };

  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const own = (await (await fetch(`${DAEMON_URL}/api/sessions?range=7d&project=${sensei!.id}`)).json()).sessions.length;
  await navigateToScreen(tauriPage, `/project/${sensei!.id}/sessions`, '[data-testid="range-chips"]', 30_000);
  await tauriPage.waitForSelector('[data-testid="session-list"], [data-screen-state="error"]', 20_000);
  const p = (await tauriPage.evaluate(rows)) as { error: string | null; rows: number };

  console.log(`\n[inspect] sessions ${JSON.stringify(o)} (daemon ${all}); sensei ${JSON.stringify(p)} (daemon ${own})`);
  expect([o.error, p.error]).toEqual([null, null]);
  expect([o.rows, p.rows]).toEqual([all, own]);
});
