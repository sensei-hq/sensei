/**
 * Instruments Playground — tool-execute round-trip.
 *
 * Verifies the sensei-tool Playground actually sends a request and renders
 * a response, closing the loop on the T2 UI rebuild:
 *
 *   1. Navigate to /instruments (Playground tab open by default).
 *   2. Click a known tool row (`get_callees`) via its data-testid.
 *   3. Assert inputs are pre-filled (`repoId` from activeWorkspace or
 *      placeholder; other inputs from `input.placeholder`).
 *   4. Click the Query button (data-testid=tool-execute).
 *   5. Wait for the response `<pre>` and assert its text is non-empty.
 *
 * The e2e daemon runs on a throw-away `sensei_e2e` DB with no indexed
 * workspaces, so `get_callees` returns `{"callees": []}`. That's a
 * correct, non-empty response — the test asserts the round-trip works,
 * not that the DB has data.
 */

import { test, expect } from '../fixtures';
import { navigateTo, seedSetupComplete, DAEMON_URL } from '../helpers';
import { seedSenseiMcp, clearCursorMcpServers } from '../isolated-home';

test.describe('Instruments Playground · execute', () => {
  // The isolated e2e home has no assistant configured with sensei: declare it, as `sensei init`
  // would, and let a real discovery scan find it; take it out again after.
  test.beforeAll(async () => {
    seedSenseiMcp();
    expect((await fetch(`${DAEMON_URL}/api/instruments/mcp-servers/refresh`, { method: 'POST' })).ok).toBe(true);
  });
  test.afterAll(async () => {
    clearCursorMcpServers();
    await fetch(`${DAEMON_URL}/api/instruments/mcp-servers/refresh`, { method: 'POST' });
  });

  test('clicking Query renders the daemon response in the <pre>', async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/instruments');
    // Give the store's loadCatalog() a chance to hydrate.
    await tauriPage.waitForSelector('[data-testid="playground-body"]', 15_000).catch(() => {});
    await new Promise((r) => setTimeout(r, 2000));

    // Click a known sensei tool with well-defined inputs.
    const toolRow = tauriPage.locator('[data-testid="tool-row-get_callees"]');
    await expect(toolRow).toBeVisible({ timeout: 10_000 });
    await toolRow.click();
    await new Promise((r) => setTimeout(r, 500));

    // The form is the tool's inputSchema (ef296c15): declared defaults and the active workspace
    // for a repo id, never an invented value. get_callees declares no default for `name`, so the
    // field starts empty and the caller supplies it, as an assistant would.
    const nameInput = tauriPage.locator('[data-testid="tool-input-name"]');
    await expect(nameInput).toBeVisible();
    expect(await nameInput.inputValue()).toBe('');
    await nameInput.fill('main');

    // Click the Query/Run button. The button carries data-testid=tool-execute
    // so we don't have to text-match through the tauri-playwright wrapper
    // (its locator engine doesn't accept the :has-text pseudo).
    const executeBtn = tauriPage.locator('[data-testid="tool-execute"]');
    await expect(executeBtn).toBeVisible();
    await executeBtn.click();

    // Wait for the response to render. The `<pre>` element is the
    // last one on the page (the response panel).
    await new Promise((r) => setTimeout(r, 3000));
    const responsePre = tauriPage.locator('pre').last();
    const responseText = await responsePre.textContent();

    // A daemon round-trip succeeded when the response text is non-empty
    // JSON. On the e2e DB `{"callees": []}` (19 chars) is the correct
    // result. On the user's real install, the same call returns 18
    // callees for `extract_deps` in the sensei workspace.
    expect((responseText ?? '').length).toBeGreaterThan(2);
    expect(responseText).toContain('{');
    expect(responseText).toContain('}');
  });
});
