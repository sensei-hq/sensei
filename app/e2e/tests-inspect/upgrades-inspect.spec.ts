/**
 * #288 against the REAL daemon: both Upgrades lanes show the daemon's reads —
 * the local count equals the installable pending recs fanned out here, the
 * dōjō count equals `/api/upgrades` (or, if the daemon cannot reach the dōjō,
 * its lane says so) — with no project left unread.
 * Run: `bun run test:inspect -- upgrades`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

const INSTALLABLE = new Set(['write_skill', 'create_agent', 'revise_rule', 'enrich_memory', 'audit_stale']);

test('upgrades shows the daemon reads', async ({ tauriPage }) => {
  test.setTimeout(180_000);
  const projects: { id: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const recs = await Promise.all(
    projects.map(async (p) => (await fetch(`${DAEMON_URL}/api/projects/${p.id}/recommendations?status=pending`)).json()),
  );
  const local = recs.flat().filter((r: { action_type?: string; actionType?: string }) =>
    INSTALLABLE.has(r.action_type ?? r.actionType ?? '')).length;
  const dojoRes = await fetch(`${DAEMON_URL}/api/upgrades`);
  const dojo = dojoRes.ok ? (await dojoRes.json()).artifacts.length : null;

  await navigateToScreen(tauriPage, '/upgrades', '[data-lane="local"]', 60_000);
  const seen = (await tauriPage.evaluate(`({
    total: document.querySelector('[data-upgrades-total]')?.getAttribute('data-upgrades-total') ?? null,
    dojo: document.querySelector('[data-dojo-total]')?.getAttribute('data-dojo-total') ?? null,
    dojoError: document.querySelector('[data-lane="dojo"] [data-screen-state="error"]')?.textContent?.trim() ?? null,
    localError: document.querySelector('[data-lane="local"] [data-screen-state="error"]')?.textContent?.trim() ?? null,
    unread: document.querySelectorAll('[data-unread-projects]').length,
  })`)) as Record<string, string | number | null>;
  console.log(`\n[inspect] upgrades ${JSON.stringify(seen)} (daemon: local ${local}, dojo ${dojo ?? `HTTP ${dojoRes.status}`})`);
  expect([seen.localError, seen.unread]).toEqual([null, 0]);
  expect(seen.total).toBe(String(local));
  if (dojo === null) expect(seen.dojoError).not.toBeNull();
  else expect([seen.dojo, seen.dojoError]).toEqual([String(dojo), null]);
});
