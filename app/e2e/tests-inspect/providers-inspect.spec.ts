/**
 * #286 against the REAL daemon: Settings → Providers lists the routers the
 * daemon reports, and shows no error state when the read succeeds. (A failed
 * read is the error state, not an empty list — covered by the wizard-state spec.)
 * Run: `bun run test:inspect -- providers`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

test('providers lists the daemon routers', async ({ tauriPage }) => {
  const { routers }: { routers: { id: string }[] } = await (await fetch(`${DAEMON_URL}/api/gateway/routers`)).json();
  await navigateToScreen(tauriPage, '/settings/providers', '[data-testid^="router-card-"]', 20_000);
  const cards = (await tauriPage.evaluate(
    `Array.from(document.querySelectorAll('[data-testid^="router-card-"]')).map((e) => e.getAttribute('data-testid').slice(12))`,
  )) as string[];
  const errorShown = (await tauriPage.evaluate(`!!document.querySelector('[data-testid="inference-error"]')`)) as boolean;
  console.log(`\n[inspect] providers: ${cards.length} cards (${cards.join(' ')}); daemon reports ${routers.length}; error state: ${errorShown}`);
  expect(cards.sort()).toEqual(routers.map((r) => r.id).sort());
  expect(errorShown).toBe(false);
});
