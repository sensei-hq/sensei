/**
 * #293 against the REAL daemon: Observatory and workspace Sessions are one
 * screen. Each shows the daemon's 7-day sessions — the Observatory across
 * workspaces, a workspace's window for that workspace alone — with no error state.
 * Read-only. Run: `bun run test:inspect -- sessions-screen`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();
const totals = `({
  line: document.querySelector('[data-testid="quality-tally"]')?.parentElement?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
  rows: document.querySelectorAll('[data-testid="session-list"] > *').length,
  errors: document.querySelectorAll('[data-screen-state="error"]').length,
})`;

test('both sessions screens show the daemon sessions', async ({ tauriPage }) => {
  const all: Array<{ workspace: string }> = (await json('/api/sessions?range=7d')).sessions;
  const workspaces = new Set(all.map((s) => s.workspace)).size;
  const list: Array<{ id: string; name: string }> = await json('/api/workspaces');
  const sensei = list.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei workspace on this daemon');
  const own: unknown[] = (await json(`/api/sessions?range=7d&workspace=${sensei!.id}`)).sessions;

  await navigateToScreen(tauriPage, '/sessions', '[data-testid="quality-tally"], [data-screen-state="error"]', 30_000);
  const obs = (await tauriPage.evaluate(totals)) as { line: string; rows: number; errors: number };
  await navigateToScreen(tauriPage, `/workspace/${sensei!.id}/sessions`, '[data-testid="workspace-sessions"] [data-testid="quality-tally"], [data-screen-state="error"]', 30_000);
  const proj = (await tauriPage.evaluate(totals)) as { line: string; rows: number; errors: number };

  console.log(`\n[inspect] sessions: observatory ${JSON.stringify(obs)} (daemon ${all.length} across ${workspaces}); sensei ${JSON.stringify(proj)} (daemon ${own.length})`);
  const s = (n: number) => (n === 1 ? '' : 's');
  expect(obs.errors + proj.errors).toBe(0);
  expect(obs.line).toContain(`${all.length} session${s(all.length)} across ${workspaces} workspace${s(workspaces)}`);
  expect(obs.rows).toBe(all.length);
  expect(proj.line).toContain(`${own.length} session${s(own.length)} · median`);
  expect(proj.line).not.toContain('across');
  expect(proj.rows).toBe(own.length);
});
