/**
 * #288 against the REAL daemon: the Settings pages that read the daemon show
 * its answers — General's form, the installed extensions, the inference role
 * assignments — with no error state. Read from the DOM: the sign-in overlay
 * (#283) is modal while it is up. Run: `bun run test:inspect -- settings-reads`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const present = (sel: string) => `!!document.querySelector('${sel}')`;
const errors = `document.querySelectorAll('[data-screen-state="error"], [data-testid="settings-inference-error"]').length`;

test('settings pages show the daemon reads', async ({ tauriPage }) => {
  const installed: unknown[] = await (await fetch(`${DAEMON_URL}/api/install/installed`)).json();
  const { chains } = await (await fetch(`${DAEMON_URL}/api/gateway/chains`)).json();
  const assigned = new Set(chains.filter((c: { role: string | null }) => c.role).map((c: { role: string }) => c.role)).size;

  await navigateTo(tauriPage, '/settings/general');
  await expect.poll(() => tauriPage.evaluate(present('[data-testid="settings-general"]')), { timeout: 30_000 }).toBe(true);
  const general = (await tauriPage.evaluate(errors)) as number;

  await navigateTo(tauriPage, '/settings/extensions');
  await expect.poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-testid^="ext-row-"]').length`), { timeout: 30_000 }).toBe(installed.length);
  const ext = (await tauriPage.evaluate(errors)) as number;

  await navigateTo(tauriPage, '/settings/inference');
  await expect.poll(() => tauriPage.evaluate(present('[data-testid^="inference-role-"]')), { timeout: 30_000 }).toBe(true);
  const inf = (await tauriPage.evaluate(errors)) as number;
  const rolesWithChain = (await tauriPage.evaluate(
    `[...document.querySelectorAll('[data-testid^="inference-chain-members-"]')].length`,
  )) as number;

  console.log(`\n[inspect] errors general/ext/inference ${[general, ext, inf]}; extensions ${installed.length}; roles with a chain ${rolesWithChain} (daemon ${assigned})`);
  expect([general, ext, inf]).toEqual([0, 0, 0]);
  expect(rolesWithChain).toBe(assigned);
});
