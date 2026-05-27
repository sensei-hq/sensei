/**
 * Detail pane — clicking a memory title shows content and outcomes.
 *
 * Seeds an active memory with a recorded "applied" outcome, navigates to the
 * Active tab, clicks the title, and verifies both detail-content and
 * detail-outcomes are rendered.
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

async function saveActiveMemory(projectId: string, title: string): Promise<string> {
  const res = await fetch(`${DAEMON_URL}/api/knowledge/memories`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      project_id: projectId,
      scope: 'project',
      type: 'convention',
      title,
      content: 'pick me — has outcomes',
      tags: ['e2e-test'],
    }),
  });
  const data = await res.json();
  if (!res.ok || !data.id) throw new Error('Failed to save memory: ' + JSON.stringify(data));
  return data.id;
}

async function recordApplied(memoryId: string): Promise<void> {
  const res = await fetch(`${DAEMON_URL}/api/knowledge/outcomes`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ outcomes: [{ memory_id: memoryId, outcome: 'applied' }] }),
  });
  if (!res.ok) throw new Error('Failed to record outcome');
}

test.describe('Learnings — detail pane', () => {
  test('clicking a memory shows content + outcomes', async ({ tauriPage }) => {
    const projectId = await getAnyProjectId();
    const title = 'e2e-detail-' + Date.now();
    const memId = await saveActiveMemory(projectId, title);
    await recordApplied(memId);

    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/learnings');

    // Switch to Active tab (page defaults to Triage if proposals exist, so be explicit).
    await tauriPage.locator('[data-testid="tab-active"]').click();

    const row = tauriPage.locator('[data-testid="active-row"]').filter({ hasText: title });
    await expect(row).toBeVisible({ timeout: 10_000 });

    // The title-btn is the first <button> inside the row.
    await row.locator('button').first().click();

    await expect(tauriPage.locator('[data-testid="detail-content"]')).toContainText('pick me', { timeout: 5_000 });
    await expect(tauriPage.locator('[data-testid="detail-outcomes"]')).toContainText('applied', { timeout: 5_000 });
  });
});
