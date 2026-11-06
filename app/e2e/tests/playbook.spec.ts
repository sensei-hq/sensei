/**
 * The playbook API, without a screen (kept from the Intake spec's daemon test).
 *
 * Intake's screen is gone (D90: Relay replaced it), but the daemon's playbook
 * endpoints still exist and are still workspace-scoped: a run always happens IN a
 * workspace, so a workspace-less recommend is an explicit error, never a fabricated
 * global recommendation.
 */
import { test, expect } from '../fixtures';
import { daemonGet, daemonPost } from '../helpers';

test('daemon recommend is workspace-scoped (accepts workspace_id, refuses a workspace-less run)', async () => {
  const workspaces = await daemonGet<Array<{ id: string }>>('/api/workspaces');
  if (!workspaces.length) throw new Error('e2e daemon has no workspaces to scope a run to');
  const id = workspaces[0].id;

  const guide = await daemonGet<Record<string, unknown>>('/api/playbook/guide');
  expect(guide).toHaveProperty('playbooks');
  expect(guide).toHaveProperty('axes');
  expect(guide).toHaveProperty('frame');

  const rec = await daemonPost<Record<string, unknown>>('/api/playbook/recommend', {
    chunk: 'add a small feature flag to gate the new panel',
    preview: true,
    workspace_id: id,
  });
  expect(rec).toHaveProperty('playbook');
  expect(rec).toHaveProperty('lifecycle');
  expect(rec).toHaveProperty('intent');
  expect(rec).toHaveProperty('risk');
  expect(rec).toHaveProperty('auto_select');

  const err = await daemonPost<Record<string, unknown>>('/api/playbook/recommend', {
    chunk: 'add a small feature flag to gate the new panel',
    preview: true,
  });
  expect(err).toHaveProperty('error');
});
