/**
 * #288 against the REAL daemon: Today renders what the daemon reports — its
 * maturity and its 14-day FTR — and not an error or the early fallback. Run:
 * `bun run test:inspect -- today`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

test('today renders the daemon read, not a fallback', async ({ tauriPage }) => {
  const today = await (await fetch(`${DAEMON_URL}/api/observatory/today`)).json();
  const ftr = await (await fetch(`${DAEMON_URL}/api/observatory/ftr`)).json();

  await navigateToScreen(tauriPage, '/', '[data-mode="mature"], [data-mode="early"], [data-screen-state="error"]', 20_000);
  const seen = (await tauriPage.evaluate(`({
    // The theme root carries a data-mode too (light/dark); Today's is the maturity.
    mode: document.querySelector('[data-mode="mature"], [data-mode="early"]')?.getAttribute('data-mode') ?? null,
    error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
    ftrError: !!document.querySelector('[data-ftr-error]'),
    header: document.querySelector('[data-ftr-header]')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
  })`)) as { mode: string | null; error: string | null; ftrError: boolean; header: string | null };
  console.log(`\n[inspect] today: ${JSON.stringify(seen)} (daemon: ${today.dataMaturity}, ftr14d ${ftr.ftr14d})`);
  expect(seen.error).toBeNull();
  expect(seen.ftrError).toBe(false);
  expect(seen.mode).toBe(today.dataMaturity);
  if (ftr.ftr14d != null) expect(seen.header).toContain(String(Math.round(ftr.ftr14d * 100)));
});
