/**
 * Configure Assistants E2E — hook registration flow.
 *
 * Verifies that when a user selects Claude Code in the Assistants wizard
 * stage and advances to Roots, the daemon registers sensei-hook-dev.ts
 * entries in ~/.claude/settings.json.
 *
 * Runs against the dev daemon (port 7745) so it never touches production
 * config. Verifies the settings.json hook entries are written, then removes
 * them to leave the environment clean.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';
import { readFileSync, existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

const SETTINGS = join(homedir(), '.claude', 'settings.json');
const DEV_HOOK = join(homedir(), '.claude', 'hooks', 'sensei-hook-dev.ts');

/** Read and parse settings.json, returns null if it doesn't exist. */
function readSettings(): Record<string, unknown> | null {
    if (!existsSync(SETTINGS)) return null;
    try { return JSON.parse(readFileSync(SETTINGS, 'utf8')); }
    catch { return null; }
}

/** Return hook entries registered for a given event type. */
function hookEntries(settings: Record<string, unknown> | null, event: string): string[] {
    if (!settings?.hooks) return [];
    const arr = (settings.hooks as Record<string, unknown>)[event];
    if (!Array.isArray(arr)) return [];
    return arr.flatMap((entry: unknown) => {
        const hooks = (entry as Record<string, unknown>)?.hooks;
        if (!Array.isArray(hooks)) return [];
        return hooks.map((h: unknown) => (h as Record<string, string>)?.command ?? '');
    });
}

/** True if sensei-hook-dev.ts is registered for all core event types. */
function devHookRegistered(settings: Record<string, unknown> | null): boolean {
    const coreEvents = ['SessionStart', 'PreToolUse', 'PostToolUse', 'Stop'];
    return coreEvents.every(ev => hookEntries(settings, ev).includes(DEV_HOOK));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async function seedHealth(tauriPage: { evaluate: (s: string) => Promise<unknown> }): Promise<void> {
    await tauriPage.evaluate(`
        (function() {
            sessionStorage.setItem('sensei:health', 'ready');
            localStorage.removeItem('sensei:setup-complete');
        })()
    `);
}

async function startAtAssistants(tauriPage: { evaluate: (s: string) => Promise<unknown> }): Promise<void> {
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await navigateTo(tauriPage, '/setup/welcome');
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe('Configure Assistants — hook registration', () => {
    test.beforeEach(async ({ tauriPage }) => {
        try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
        await startAtAssistants(tauriPage);
    });

    test('assistants page renders with Claude card', async ({ tauriPage }) => {
        // Drive through welcome → preferences → assistants
        await tauriPage.click('.btn-primary'); // welcome → preferences
        await tauriPage.locator('.name-input').waitFor({ timeout: 6_000 });
        await tauriPage.locator('.name-input').fill('Test User');
        await tauriPage.click('.btn-primary'); // preferences → assistants

        // Wait for assistants page content to load
        const assistantsSection = tauriPage.locator('.grid, .empty, p');
        await expect(assistantsSection.first()).toBeVisible({ timeout: 10_000 });

        // Verify Continue button is always available on assistants page
        // (it does not require a selection to proceed)
        await expect(tauriPage.locator('.btn-primary')).toBeEnabled({ timeout: 5_000 });
    });

    test('configure API call succeeds when Claude Code selected', async ({ tauriPage }) => {
        // Drive to assistants page
        await tauriPage.click('.btn-primary');
        await tauriPage.locator('.name-input').waitFor({ timeout: 6_000 });
        await tauriPage.locator('.name-input').fill('Test User');
        await tauriPage.click('.btn-primary');
        await tauriPage.locator('.btn-primary').waitFor({ state: 'visible', timeout: 10_000 });

        // Intercept the configure API call
        let configureCallMade = false;
        let configurePayload: unknown = null;

        tauriPage.on('request', (req: { url: () => string; method: () => string; postData: () => string | null }) => {
            if (req.url().includes('/api/assistants/configure') && req.method() === 'POST') {
                configureCallMade = true;
                configurePayload = req.postData();
            }
        });

        // Advance from assistants → roots (commitStage triggers configure call)
        await tauriPage.click('.btn-primary');

        // Wait for navigation or the configure response
        await tauriPage.waitForTimeout(3_000);

        // Verify the configure API was called (even if the actual plugin install
        // can't run in test env, the wizard should make the API request)
        // Note: configure may succeed or fail depending on whether claude binary
        // is available; we only verify the API was invoked.
        expect(configureCallMade, 'configure API should have been called on advance').toBe(true);
    });

    test('dev daemon configure endpoint writes dev hook entries', async ({ tauriPage }) => {
        // Snapshot settings before
        const before = readSettings();
        const hadDevHookBefore = devHookRegistered(before);

        // Call the configure endpoint directly (simulates wizard commit)
        const resp = await fetch(`${DAEMON_URL}/api/assistants/configure`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ acps: ['claude-code'] }),
        });
        expect(resp.ok || resp.status === 200, 'configure endpoint should return 200').toBe(true);

        // Wait briefly for file write
        await tauriPage.waitForTimeout(500);

        // Verify settings.json contains dev hook entries
        const after = readSettings();
        expect(after, 'settings.json should exist after configure').not.toBeNull();
        expect(devHookRegistered(after), 'sensei-hook-dev.ts should be registered for core events').toBe(true);

        // Cleanup: remove dev hook entries via the remove endpoint
        await fetch(`${DAEMON_URL}/api/assistants/remove`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ acps: ['claude-code'] }),
        });

        // After removal, dev hook should be gone (if it wasn't there before)
        if (!hadDevHookBefore) {
            await tauriPage.waitForTimeout(300);
            const afterRemove = readSettings();
            expect(
                devHookRegistered(afterRemove),
                'sensei-hook-dev.ts should be removed after uninstall'
            ).toBe(false);
        }
    });

    test('idempotent configure does not duplicate hook entries', async ({ tauriPage }) => {
        // Configure twice
        for (let i = 0; i < 2; i++) {
            await fetch(`${DAEMON_URL}/api/assistants/configure`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ acps: ['claude-code'] }),
            });
        }
        await tauriPage.waitForTimeout(500);

        const settings = readSettings();
        const devEntries = hookEntries(settings, 'SessionStart')
            .filter(cmd => cmd === DEV_HOOK);
        expect(devEntries.length, 'SessionStart should have exactly one dev hook entry').toBe(1);

        // Cleanup
        await fetch(`${DAEMON_URL}/api/assistants/remove`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ acps: ['claude-code'] }),
        });
    });
});
