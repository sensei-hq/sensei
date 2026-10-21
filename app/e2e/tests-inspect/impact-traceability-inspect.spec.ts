/**
 * #288 against the REAL daemon: Impact and Traceability, observatory-wide and
 * for this repository's project, show the daemon's totals — fanned out here
 * independently — with no error state and no project left unread.
 * Run: `bun run test:inspect -- impact-traceability`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();

test('impact and traceability show the daemon totals', async ({ tauriPage }) => {
  test.setTimeout(180_000);
  const projects: { id: string; name: string }[] = await json('/api/projects');
  const impact = (await Promise.all(projects.map((p) => json(`/api/projects/${p.id}/impact`)))).flat().length;
  const drift = (await Promise.all(projects.map((p) => json(`/api/projects/${p.id}/drift`))))
    .reduce((n, d) => n + d.items.length, 0);

  const read = async () =>
    (await tauriPage.evaluate(`({
      impact: document.querySelector('[data-impact-total]')?.getAttribute('data-impact-total') ?? null,
      trace: document.querySelector('[data-traceability-total]')?.getAttribute('data-traceability-total') ?? null,
      bad: document.querySelectorAll('[data-screen-state="error"], [data-unread-projects]').length,
    })`)) as { impact: string | null; trace: string | null; bad: number };

  await navigateToScreen(tauriPage, '/impact', '[data-impact-total], [data-screen-state="error"]', 60_000);
  const i = await read();
  await navigateToScreen(tauriPage, '/traceability', '[data-traceability-total], [data-screen-state="error"]', 60_000);
  const t = await read();
  console.log(`\n[inspect] impact ${JSON.stringify(i)} (daemon ${impact}); traceability ${JSON.stringify(t)} (daemon ${drift})`);
  expect([i.bad, t.bad]).toEqual([0, 0]);
  expect(i.impact).toBe(String(impact));
  expect(t.trace).toBe(String(drift));

  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');
  const own = await json(`/api/projects/${sensei!.id}/drift`);
  await navigateToScreen(tauriPage, `/project/${sensei!.id}/traceability`, '[data-testid="drift-scan-button"]', 30_000);
  const header = (await tauriPage.evaluate(
    `document.querySelector('[data-testid="drift-scan-button"]')?.parentElement?.textContent?.replace(/\\s+/g, ' ').trim() ?? null`,
  )) as string;
  const errs = (await tauriPage.evaluate(`document.querySelectorAll('[data-screen-state="error"]').length`)) as number;
  console.log(`[inspect] sensei traceability "${header}" (daemon ${own.total}/${own.drifted}/${own.broken})`);
  expect(errs).toBe(0);
  expect(header).toContain(`${own.total} tracked · ${own.drifted} drifted · ${own.broken} broken`);

  // #293: the project Impact tab — its measured total and the impact log's
  // pending entries are the daemon's, and its log form is idle.
  const measured = (await json(`/api/projects/${sensei!.id}/impact`)).length;
  const { verdicts } = await json(`/api/projects/${sensei!.id}/impact-verdicts`);
  const pending = verdicts.filter((v: { verdict: string }) => v.verdict === 'pending').length;
  await navigateToScreen(tauriPage, `/project/${sensei!.id}/impact`, '[data-impact-total], [data-screen-state="error"]', 30_000);
  const tab = (await tauriPage.evaluate(`({
    total: document.querySelector('[data-impact-total]')?.getAttribute('data-impact-total') ?? null,
    pending: document.querySelectorAll('[data-testid^="impact-pending-"]').length - (document.querySelector('[data-testid="impact-pending-list"]') ? 1 : 0),
    errors: document.querySelectorAll('[data-screen-state="error"], [data-testid="impact-log-error"]').length,
    button: !!document.querySelector('[data-testid="impact-log-button"]'),
  })`)) as { total: string | null; pending: number; errors: number; button: boolean };
  console.log(`[inspect] sensei impact ${JSON.stringify(tab)} (daemon ${measured} measured, ${pending} pending of ${verdicts.length})`);
  expect(tab).toEqual({ total: String(measured), pending, errors: 0, button: true });
});
