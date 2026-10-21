/**
 * #288 against the REAL daemon: the rail's project badge is the daemon's
 * project count (a failed read leaves no badge, never 0).
 * Run: `bun run test:inspect -- project-count`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

test('the rail badge is the daemon project count', async ({ tauriPage }) => {
  const projects: unknown[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  await navigateTo(tauriPage, '/');
  await expect
    .poll(
      () => tauriPage.evaluate(`[...document.querySelectorAll('a[href="/projects"]')].map((a) => a.textContent.replace(/\\s+/g, ' ').trim()).join(' | ')`),
      { timeout: 30_000 },
    )
    .toContain(String(projects.length));
  console.log(`\n[inspect] project badge shows ${projects.length}`);
});
