/**
 * Reject → Archive flow.
 *
 * Seeds a proposal, rejects it from the Triage tab, then switches to the
 * Archive tab and confirms the row appears there.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

/** Mark setup complete in both daemon and the running app's in-memory config. */
async function seedSetupComplete(tauriPage: any): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function() {
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

async function getAnyProjectId(): Promise<string> {
  const res = await fetch(`${DAEMON_URL}/api/projects`);
  const projects: { id: string }[] = await res.json();
  if (!projects.length) throw new Error('No projects in dev DB');
  return projects[0].id;
}

async function createProposal(projectId: string, title: string): Promise<string> {
  const res = await fetch(`${DAEMON_URL}/api/knowledge/proposals`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      project_id: projectId,
      scope: 'project',
      type: 'convention',
      title,
      content: 'reject me',
      triage_signal: 'correction',
      tags: ['e2e-test'],
    }),
  });
  const data = await res.json();
  if (!res.ok || !data.id) throw new Error('Failed to create proposal: ' + JSON.stringify(data));
  return data.id;
}

test.describe('Learnings — Reject → Archive', () => {
  test('rejecting a proposal moves it to the Archive tab', async ({ tauriPage }) => {
    const projectId = await getAnyProjectId();
    const uniqueTitle = 'e2e-reject-' + Date.now();
    await createProposal(projectId, uniqueTitle);

    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/learnings');

    // Filter by exact unique title — there may be leftover rows from prior runs.
    const triageRow = tauriPage.locator('[data-testid="triage-row"]').filter({ hasText: uniqueTitle });
    await expect(triageRow).toBeVisible({ timeout: 10_000 });
    await triageRow.locator('[data-testid="reject-btn"]').click();
    // After reject, this specific row should no longer be in the triage list.
    await expect(triageRow).toBeHidden({ timeout: 5_000 });

    await tauriPage.locator('[data-testid="tab-archive"]').click();
    const archiveRow = tauriPage.locator('[data-testid="archive-row"]').filter({ hasText: uniqueTitle });
    await expect(archiveRow).toBeVisible({ timeout: 5_000 });
  });
});
