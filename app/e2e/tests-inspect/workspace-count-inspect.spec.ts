/**
 * #288 against the REAL daemon: the rail's workspace badge is the daemon's
 * workspace count (a failed read leaves no badge, never 0).
 * Run: `bun run test:inspect -- workspace-count`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

test('the rail badge is the daemon workspace count', async ({ tauriPage }) => {
  const workspaces: unknown[] = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  await navigateTo(tauriPage, '/');
  await expect
    .poll(
      () => tauriPage.evaluate(`[...document.querySelectorAll('a[href="/workspaces"]')].map((a) => a.textContent.replace(/\\s+/g, ' ').trim()).join(' | ')`),
      { timeout: 30_000 },
    )
    .toContain(String(workspaces.length));
  console.log(`\n[inspect] workspace badge shows ${workspaces.length}`);
});
