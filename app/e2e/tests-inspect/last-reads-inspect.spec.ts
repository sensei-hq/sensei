/**
 * #288 against the REAL daemon: the last converted reads land — Learnings'
 * active count, the knowledge sources, the relay crew — with no error state.
 * Read from the DOM: the sign-in overlay (#283) is modal while it is up.
 * Run: `bun run test:inspect -- last-reads`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();
const errors = `document.querySelectorAll('[data-screen-state="error"]').length`;

test('the last converted reads land', async ({ tauriPage }) => {
  // The Active tab is these four statuses together (memoryState STATUSES_BY_TAB).
  let active = 0;
  for (const status of ['active', 'reinforced', 'challenged', 'battle_tested']) {
    active += (await json(`/api/knowledge/memories?status=${status}&limit=500`)).memories.length;
  }
  const sources = (await json('/api/knowledge/sources')).sources.length;
  const workspaces: { id: string; name: string }[] = await json('/api/workspaces');
  const sensei = workspaces.find((p) => p.name === 'sensei')!;

  await navigateTo(tauriPage, '/learnings');
  await expect
    .poll(() => tauriPage.evaluate(`document.body.textContent.match(/Active\\s*(\\d+)/)?.[1] ?? null`), { timeout: 30_000 })
    .not.toBeNull();
  const shownActive = (await tauriPage.evaluate(`document.body.textContent.match(/Active\\s*(\\d+)/)?.[1]`)) as string;
  const learn = (await tauriPage.evaluate(errors)) as number;

  await navigateTo(tauriPage, '/knowledge-sources');
  await new Promise((r) => setTimeout(r, 2000));
  const cards = (await tauriPage.evaluate(`document.querySelectorAll('[data-testid="source-card"]').length`)) as number;
  const know = (await tauriPage.evaluate(`document.querySelectorAll('.text-error').length`)) as number;

  // Relay is the last converted read: the crew and its queue from one view.
  await navigateTo(tauriPage, `/workspace/${sensei.id}/relay`);
  await expect.poll(() => tauriPage.evaluate(`!!document.querySelector('[data-screen="relay"]')`), { timeout: 30_000 }).toBe(true);
  const relay = (await tauriPage.evaluate(errors)) as number;

  console.log(`\n[inspect] learnings active ${shownActive} (daemon ${active}); sources ${cards} (daemon ${sources}); relay errors ${relay}; errors ${[learn, know, relay]}`);
  expect([learn, know, relay]).toEqual([0, 0, 0]);
  expect(Number(shownActive)).toBe(active);
  expect(cards).toBe(sources);
});
