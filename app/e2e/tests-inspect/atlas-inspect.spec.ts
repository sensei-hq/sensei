/**
 * #288 against the REAL daemon: this repository's Atlas renders the daemon's
 * call-flow counts for its repo, and no error state. Run:
 * `bun run test:inspect -- atlas`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

test('atlas shows the daemon call-flow counts', async ({ tauriPage }) => {
  const workspaces: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const sensei = workspaces.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei workspace on this daemon');

  await navigateToScreen(tauriPage, `/workspace/${sensei!.id}/atlas`, '[data-screen="atlas"], [data-screen-state="error"]', 60_000);
  const seen = (await tauriPage.evaluate(`({
    repo: document.querySelector('[data-screen="atlas"]')?.getAttribute('data-atlas-repo') ?? null,
    error: document.querySelector('[data-screen-state="error"]')?.textContent?.trim() ?? null,
    stats: Array.from(document.querySelectorAll('[data-screen="atlas"] .font-mono.text-lg')).map((e) => e.textContent.trim()),
  })`)) as { repo: string | null; error: string | null; stats: string[] };
  const flow = await (await fetch(`${DAEMON_URL}/api/graph/call-flow?repoId=${encodeURIComponent(seen.repo ?? '')}`)).json();
  console.log(`\n[inspect] atlas ${JSON.stringify(seen)} (daemon: ${flow.moduleCount}/${flow.exportCount}/${flow.callCount})`);
  expect(seen.error).toBeNull();
  expect(seen.stats).toEqual([String(flow.moduleCount), String(flow.exportCount), String(flow.callCount)]);
});
