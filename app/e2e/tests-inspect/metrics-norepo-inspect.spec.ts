/**
 * #144 against the REAL daemon: a project with no repository says why it has no
 * metrics instead of "not computed yet"; a project with one shows its metrics.
 * Run: `bun run test:inspect -- metrics-norepo`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

test('a repo-less project says why it has no metrics', async ({ tauriPage }) => {
  const projects: { id: string; name: string; repos_count: number }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const none = projects.find((p) => p.repos_count === 0);
  const some = projects.find((p) => p.name === 'sensei' && p.repos_count > 0);
  test.skip(!none || !some, 'needs one project with and one without a repository');

  await navigateToScreen(tauriPage, `/project/${none!.id}/metrics`, '[data-component="metrics-empty"]', 20_000);
  const kind = await tauriPage.evaluate(`document.querySelector('[data-component="metrics-empty"]').getAttribute('data-empty-kind')`);
  const text = (await tauriPage.evaluate(`document.querySelector('[data-component="metrics-empty"]').textContent.trim()`)) as string;
  console.log(`\n[inspect] repo-less project: ${kind} — "${text}"`);
  expect(kind).toBe('no-repository');

  await navigateToScreen(tauriPage, `/project/${some!.id}/metrics`, '[data-component="project-shell"]', 20_000);
  // The metrics load fetches a series per metric, so wait for either outcome.
  const state = () =>
    tauriPage.evaluate(
      `document.querySelector('[data-component="metrics-grid"]') ? 'metrics' : document.querySelector('[data-component="metrics-empty"]') ? 'empty' : 'loading'`,
    ) as Promise<string>;
  // Navigation is client-side: the previous page's notice can still be in the
  // DOM, so wait for THIS project's outcome — its signals, or its own notice.
  const kindNow = () =>
    tauriPage.evaluate(
      `location.pathname + ' ' + (document.querySelector('[data-component="metrics-empty"]')?.getAttribute('data-empty-kind') ?? '-')`,
    ) as Promise<string>;
  await expect.poll(state, { timeout: 60_000 }).toBe('metrics');
  console.log(`[inspect] after the wait: ${await kindNow()}`);
  const empty = (await state()) === 'empty';
  console.log(`[inspect] sensei project: empty notice shown: ${empty}`);
  expect(empty).toBe(false);
});
