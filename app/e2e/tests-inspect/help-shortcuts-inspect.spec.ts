/**
 * #135 against the real app: Help → Keyboard Shortcuts opens /help, and /help
 * lists only the shortcuts that are bound. The native menu is clicked through
 * System Events, which needs Accessibility permission for the terminal — the
 * test says so, rather than passing, when it is refused.
 * Run: `bun run test:inspect -- help-shortcuts`.
 */
import { expect, test } from '../fixtures';
import { navigateToScreen } from '../helpers';
import { execFileSync } from 'child_process';

test('help lists only bound shortcuts, and the menu item opens it', async ({ tauriPage }) => {
  await navigateToScreen(tauriPage, '/', '[data-testid="observatory-today"], main');
  let clicked = true;
  try {
    execFileSync('osascript', [
      '-e',
      'tell application "System Events" to tell (first process whose name contains "sensei") to click menu item "Keyboard Shortcuts" of menu "Help" of menu bar 1',
    ]);
  } catch (e) {
    clicked = false;
    console.log(`\n[inspect] menu click refused (Accessibility?): ${String(e).split('\n')[0]}`);
  }
  if (clicked) {
    await expect.poll(async () => (await tauriPage.evaluate('window.location.pathname')) as string, {
      timeout: 10_000,
    }).toBe('/help');
    console.log('\n[inspect] Help → Keyboard Shortcuts navigated to /help');
  } else {
    await navigateToScreen(tauriPage, '/help', '[data-shortcut]');
  }
  const bindings = (await tauriPage.evaluate(
    `Array.from(document.querySelectorAll('[data-shortcut]')).map((r) => r.getAttribute('data-shortcut'))`,
  )) as string[];
  console.log(`[inspect] /help shortcuts: ${bindings.join(', ')}`);
  expect(bindings).toEqual(['mod+k', 'CmdOrCtrl+Shift+I', 'CmdOrCtrl+Shift+L', 'CmdOrCtrl+Shift+/']);
  expect(clicked, 'the native menu click must be possible for this check to be complete').toBe(true);
});
