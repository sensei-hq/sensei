/**
 * Settings is its own window (user, 2026-10-27: as a Mac app's preferences
 * are). Against the real app: the main rail's Settings opens a window labelled
 * `settings`, and the main window stays where it was — it no longer draws the
 * settings pages inside itself.
 *
 * Run: `bun run test:inspect -- settings-window`.
 */
import { expect, test } from '../fixtures';
import { navigateToScreen } from '../helpers';

const WINDOWS = `(async () => {
  const all = await window.__TAURI_INTERNALS__.invoke('plugin:webview|get_all_webviews');
  return JSON.stringify(all.map((w) => w.label ?? w.windowLabel ?? w));
})()`;

test('the rail’s Settings opens the settings window, and the main window stays put', async ({ tauriPage }) => {
  await navigateToScreen(tauriPage, '/workspaces', '[data-component="observatory-sidebar"]');
  const before = JSON.parse((await tauriPage.evaluate(WINDOWS)) as string) as string[];
  expect(before).not.toContain('settings');

  // The e2e build opens settings inside this window so other specs can reach
  // it; this spec is about the real behaviour, so it turns that off.
  await tauriPage.evaluate(`window.__SENSEI_E2E__ = false`);
  await tauriPage.evaluate(
    `document.querySelector('[data-component="observatory-sidebar"] a[href="/settings"]')?.click()`,
  );
  await expect
    .poll(async () => JSON.parse((await tauriPage.evaluate(WINDOWS)) as string) as string[], { timeout: 15_000 })
    .toContain('settings');
  expect(await tauriPage.evaluate('location.pathname')).toBe('/workspaces');
  expect(await tauriPage.evaluate(`!!document.querySelector('[data-component="settings-sidebar"]')`)).toBe(false);
  await tauriPage.evaluate(`window.__SENSEI_E2E__ = true`);
});
