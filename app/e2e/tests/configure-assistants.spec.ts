/**
 * Configure Assistants E2E — Claude Code plugin install.
 *
 * Verifies that configuring Claude Code installs and enables the sensei plugin,
 * and that removing it uninstalls it.
 *
 * Tests 1-2 drive the Tauri UI.
 * Tests 3-4 call the daemon API directly to verify install/remove/idempotency.
 *
 * The daemon runs under the isolated E2E_HOME, so the real `claude plugin`
 * commands it issues act on a throwaway ~/.claude, never the developer's (#267).
 */

import { test, expect, type TauriPageLike } from '../fixtures';
import { DAEMON_URL, startAtWelcome, clickAndExpectNav } from '../helpers';
import { senseiPluginEnabled, senseiPluginInstalls } from '../isolated-home';

async function assistants(action: 'configure' | 'remove'): Promise<Response> {
    return fetch(`${DAEMON_URL}/api/assistants/${action}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ acps: ['claude-code'] }),
    });
}

/** Welcome → assistants, through the wizard as a user would. `startAtWelcome`
 *  opens `/setup/welcome?force=1`: setup is marked complete for the observatory
 *  specs, and without `force` the wizard bounces to the observatory. */
async function goToAssistants(tauriPage: TauriPageLike): Promise<void> {
    await startAtWelcome(tauriPage);
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 10_000 });
}

// ── UI tests ──────────────────────────────────────────────────────────────────

test.describe('Configure Assistants — hook registration', () => {
    test.beforeEach(async ({ tauriPage }) => {
        try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    });

    test('assistants page is reachable and Continue is always enabled', async ({ tauriPage }) => {
        await goToAssistants(tauriPage);
        // Continue is always enabled on assistants — no required selection
        await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 5_000 });
    });

    test('assistants → roots navigates without health flash', async ({ tauriPage }) => {
        await goToAssistants(tauriPage);

        const seen: string[] = [];
        const deadline = Date.now() + 10_000;
        await tauriPage.click('[data-action="next"]');

        let reached = false;
        while (Date.now() < deadline) {
            try {
                const p = await tauriPage.evaluate(`window.location.pathname`);
                if (typeof p === 'string') {
                    seen.push(p);
                    if (p === '/setup/roots') { reached = true; break; }
                }
            } catch { /* mid-transition */ }
            await new Promise<void>(r => setTimeout(r, 80));
        }

        const flashedHealth = seen.filter(p => p === '/health');
        expect(flashedHealth, 'no redirect to /health during navigation').toHaveLength(0);
        expect(reached, 'should reach /setup/roots').toBe(true);
    });

    // ── Daemon API tests (no UI required) ─────────────────────────────────────

    test('configure installs and enables the sensei plugin; remove uninstalls it', async () => {
        test.setTimeout(180_000);
        expect((await assistants('configure')).status).toBe(200);
        expect(senseiPluginInstalls(), 'installed_plugins.json records sensei').toBe(1);
        expect(senseiPluginEnabled(), 'settings.json enables sensei@sensei-marketplace').toBe(true);

        expect((await assistants('remove')).status).toBe(200);
        expect(senseiPluginInstalls(), 'sensei uninstalled').toBe(0);
    });

    test('configure is idempotent — one install after two', async () => {
        test.setTimeout(180_000);
        for (let i = 0; i < 2; i++) expect((await assistants('configure')).status).toBe(200);
        expect(senseiPluginInstalls()).toBe(1);
        await assistants('remove');
    });
});
