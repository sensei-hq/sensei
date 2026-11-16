/**
 * Relay — the crew screen (D90), in the real built Tauri app against a fresh e2e
 * daemon.
 *
 * Relay replaced Intake as the workspace window's front door and also lives in the
 * main window: 67 of 157 repositories belong to no workspace, so a screen that only
 * existed per workspace could not show their crews. This verifies both routes render
 * the same screen and that both rails reach it.
 *
 * Seed-agnostic: the throwaway sensei_e2e DB has no crews, so the screen draws its
 * "No crew yet" band — the plumbing is what this asserts, not the copy.
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateTo, navigateToScreen, daemonGet, DAEMON_URL } from '../helpers';

async function seedSetupComplete(tauriPage: TauriPageLike): Promise<void> {
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

async function pathname(tauriPage: TauriPageLike): Promise<string> {
  return (await tauriPage.evaluate(`window.location.pathname`)) as string;
}

/** First workspace in the e2e DB — the workspace window needs a real id. */
async function firstWorkspaceId(): Promise<string> {
  const workspaces = await daemonGet<Array<{ id: string }>>('/api/workspaces');
  if (!workspaces.length) throw new Error('e2e daemon has no workspaces to scope Relay to');
  return workspaces[0].id;
}

test.describe('Relay — one screen, two places', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target
  });

  test('the main window renders Relay', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await navigateToScreen(tauriPage, '/relay', '[data-screen="relay"]');
    expect(await pathname(tauriPage)).toBe('/relay');
    await expect(tauriPage.locator('[data-screen="relay"][data-screen-shell]')).toBeVisible();
  });

  test('the workspace window renders the same screen, pre-filtered', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    const id = await firstWorkspaceId();
    await navigateToScreen(tauriPage, `/workspace/${id}/relay`, '[data-screen="relay"]');
    expect(await pathname(tauriPage)).toBe(`/workspace/${id}/relay`);
    await expect(tauriPage.locator('[data-screen="relay"][data-screen-shell]')).toBeVisible();
  });

  test('both rails link to Relay', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    const id = await firstWorkspaceId();

    await navigateToScreen(tauriPage, '/', 'a[href="/relay"]');
    expect(await tauriPage.locator('a[href="/relay"]').count()).toBeGreaterThan(0);

    await navigateToScreen(tauriPage, `/workspace/${id}/overview`, `a[href="/workspace/${id}/relay"]`);
    expect(await tauriPage.locator(`a[href="/workspace/${id}/relay"]`).count()).toBeGreaterThan(0);
  });
});
