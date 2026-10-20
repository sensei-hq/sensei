/**
 * #288 against the REAL daemon: the Insights board's column counts are the
 * daemon's `counts`, with no error state. Run: `bun run test:inspect -- insights`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

test('insights shows the daemon board', async ({ tauriPage }) => {
  const { counts } = await (await fetch(`${DAEMON_URL}/api/insights`)).json();
  await navigateToScreen(tauriPage, '/insights', '[data-triage-grid], [data-screen-state="error"]', 60_000);
  const seen = (await tauriPage.evaluate(`({
    error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
    stats: Array.from(document.querySelectorAll('.font-mono.text-lg.leading-none')).slice(0, 3).map((e) => Number(e.textContent.trim())),
  })`)) as { error: string | null; stats: number[] };
  console.log(`\n[inspect] insights ${JSON.stringify(seen)} (daemon ${JSON.stringify(counts)})`);
  expect(seen.error).toBeNull();
  expect(seen.stats).toEqual([counts.now, counts.soon, counts.settled]);
});
