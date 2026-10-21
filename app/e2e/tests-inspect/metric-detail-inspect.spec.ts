/**
 * #293 against the REAL daemon: a metric's detail chart shows the grain's
 * default window over the daemon's series (the last 14 days, gaps included),
 * the latest day with a value is selected, and moving the window re-defaults
 * the selection. Read-only. Run: `bun run test:inspect -- metric-detail`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();
const DAY = 86_400_000;

test('metric detail windows the daemon series', async ({ tauriPage }) => {
  const projects: Array<{ id: string; name: string }> = await json('/api/projects');
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const { series } = await json(`/api/projects/${sensei!.id}/metrics/ftr?grain=daily`);
  test.skip(series.length < 2, 'ftr has no history here');
  const first = Date.parse(series[0].period.slice(0, 10));
  const last = Date.parse(series[series.length - 1].period.slice(0, 10));
  const full = Math.round((last - first) / DAY) + 1;
  const window = Math.min(14, full);

  await navigateToScreen(tauriPage, `/project/${sensei!.id}/metrics/ftr`, '[data-component="chart-window"], [data-screen-state="error"]', 60_000);
  const read = `({
    label: document.querySelector('[data-component="chart-window"]')?.nextElementSibling?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
    // Only days with a value get a marker; the selected one should be the latest.
    marks: document.querySelectorAll('[data-selected]').length,
    selected: [...document.querySelectorAll('[data-selected]')].findIndex((e) => e.getAttribute('data-selected') === 'true'),
    errors: document.querySelectorAll('[data-screen-state="error"]').length,
  })`;
  const before = (await tauriPage.evaluate(read)) as { label: string; marks: number; selected: number; errors: number };

  await tauriPage.evaluate(`(() => {
    const r = document.querySelector('[data-component="chart-window"]');
    r.value = '0';
    r.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await expect.poll(() => tauriPage.evaluate(`document.querySelector('[data-component="chart-window"]')?.nextElementSibling?.textContent?.trim().startsWith('${full} of')`), { timeout: 10_000 }).toBe(true);
  const after = (await tauriPage.evaluate(read)) as { label: string; marks: number; selected: number; errors: number };

  console.log(`\n[inspect] metric detail ftr: daemon ${series.length} points over ${full} days; before ${JSON.stringify(before)}; window at 0 ${JSON.stringify(after)}`);
  const inWindow = series.filter((p: { period: string; value: number | null }) =>
    p.value != null && Date.parse(p.period.slice(0, 10)) > last - window * DAY).length;
  expect(before).toEqual({ label: `${window} of ${full} days`, marks: inWindow, selected: inWindow - 1, errors: 0 });
  const defined = series.filter((p: { value: number | null }) => p.value != null).length;
  expect(after).toEqual({ label: `${full} of ${full} days`, marks: defined, selected: defined - 1, errors: 0 });
});
