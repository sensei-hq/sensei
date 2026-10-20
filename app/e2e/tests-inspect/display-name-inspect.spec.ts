/**
 * Against the REAL daemon: with no display name saved, Settings → General shows
 * the machine's user name and saves it, as the setup wizard does.
 * Run: `bun run test:inspect -- display-name`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const userName = async () => ((await (await fetch(`${DAEMON_URL}/api/config`)).json()) as Record<string, string>).user_name ?? '';

test('an empty display name is filled from the system user name and saved', async ({ tauriPage }) => {
  test.setTimeout(120_000);
  const before = await userName();
  await navigateTo(tauriPage, '/settings/general');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelector('[data-testid="pref-display-name"]')?.value ?? null`), { timeout: 30_000 })
    .not.toBeNull();
  const shown = (await tauriPage.evaluate(`document.querySelector('[data-testid="pref-display-name"]').value`)) as string;
  await expect.poll(userName, { timeout: 10_000 }).toBe(shown);
  console.log(`\n[inspect] display name: before ${JSON.stringify(before)} → shown ${JSON.stringify(shown)}, saved ${JSON.stringify(await userName())}`);
  expect(shown.trim()).not.toBe('');
});
