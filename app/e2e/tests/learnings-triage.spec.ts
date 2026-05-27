/**
 * Triage tab — proposals appear, Accept moves out of triage, Reject moves to archive.
 *
 * Seeds the daemon directly via /api/knowledge/proposals and cleans up after.
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
  // Inject into the live appState singleton via the test handle exposed in +layout.svelte.
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
      content: 'test body for ' + title,
      triage_signal: 'revert',
      tags: ['e2e-test'],
    }),
  });
  const data = await res.json();
  if (!res.ok || !data.id) throw new Error('Failed to create proposal: ' + JSON.stringify(data));
  return data.id;
}

test.describe('Learnings — Triage tab', () => {
  let proposalId: string;
  let projectId: string;
  let uniqueTitle: string;

  test.beforeAll(async () => {
    projectId = await getAnyProjectId();
  });

  test.beforeEach(async ({ tauriPage }) => {
    uniqueTitle = 'e2e-triage-' + Date.now();
    proposalId = await createProposal(projectId, uniqueTitle);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/learnings');
  });

  test.afterEach(async () => {
    // No DELETE endpoint — row stays as proposed/rejected. Unique title keeps tests isolated.
    void proposalId;
  });

  test('proposal appears in triage and Accept removes it', async ({ tauriPage }) => {
    // Filter by exact unique title — there may be leftover rows from prior runs.
    const row = tauriPage.locator('[data-testid="triage-row"]').filter({ hasText: uniqueTitle });
    await expect(row).toBeVisible({ timeout: 10_000 });

    await row.locator('[data-testid="accept-btn"]').click();
    // After accept, this specific row should be gone from the triage list.
    await expect(row).toBeHidden({ timeout: 5_000 });
  });

  test('proposal appears in triage and Reject removes it from triage', async ({ tauriPage }) => {
    const row = tauriPage.locator('[data-testid="triage-row"]').filter({ hasText: uniqueTitle });
    await expect(row).toBeVisible({ timeout: 10_000 });

    await row.locator('[data-testid="reject-btn"]').click();
    await expect(row).toBeHidden({ timeout: 5_000 });
  });
});
