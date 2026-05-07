/**
 * Configure Assistants E2E — hook registration flow.
 *
 * Verifies that when a user advances through the Assistants wizard stage,
 * the dev daemon registers sensei-hook-dev.ts entries in ~/.claude/settings.json.
 *
 * Tests 1-2 drive the Tauri UI.
 * Tests 3-4 call the daemon API directly to verify hook write/remove/idempotency.
 *
 * Runs against the dev daemon (port 7745) — never touches production config.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';
import { readFileSync, existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

const SETTINGS  = join(homedir(), '.claude', 'settings.json');
const DEV_HOOK  = join(homedir(), '.claude', 'hooks', 'sensei-hook-dev.ts');

// ── Helpers ───────────────────────────────────────────────────────────────────

function readSettings(): Record<string, unknown> | null {
    if (!existsSync(SETTINGS)) return null;
    try { return JSON.parse(readFileSync(SETTINGS, 'utf8')); }
    catch { return null; }
}

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

function devHookRegistered(settings: Record<string, unknown> | null): boolean {
    const coreEvents = ['SessionStart', 'PreToolUse', 'PostToolUse', 'Stop'];
    return coreEvents.every(ev => hookEntries(settings, ev).includes(DEV_HOOK));
}

async function seedHealth(tauriPage: { evaluate: (s: string) => Promise<unknown> }): Promise<void> {
    await tauriPage.evaluate(`
        (function() {
            sessionStorage.setItem('sensei:health', 'ready');
            localStorage.removeItem('sensei:setup-complete');
        })()
    `);
}

async function goToAssistants(tauriPage: { evaluate: (s: string) => Promise<unknown>; click: (s: string) => Promise<void>; locator: (s: string) => unknown }): Promise<void> {
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await navigateTo(tauriPage, '/setup/welcome');
    // welcome → preferences
    await (tauriPage as import('@playwright/test').Page).click('.btn-primary');
    await expect((tauriPage as import('@playwright/test').Page).locator('.name-input')).toBeVisible({ timeout: 8_000 });
    await (tauriPage as import('@playwright/test').Page).locator('.name-input').fill('Test User');
    // preferences → assistants
    await (tauriPage as import('@playwright/test').Page).click('.btn-primary');
    await expect((tauriPage as import('@playwright/test').Page).locator('.btn-primary')).toBeEnabled({ timeout: 10_000 });
}

// ── UI tests ──────────────────────────────────────────────────────────────────

test.describe('Configure Assistants — hook registration', () => {
    test.beforeEach(async ({ tauriPage }) => {
        try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    });

    test('assistants page is reachable and Continue is always enabled', async ({ tauriPage }) => {
        await goToAssistants(tauriPage);
        // Continue is always enabled on assistants — no required selection
        await expect(tauriPage.locator('.btn-primary')).toBeEnabled({ timeout: 5_000 });
    });

    test('assistants → roots navigates without health flash', async ({ tauriPage }) => {
        await goToAssistants(tauriPage);

        const seen: string[] = [];
        const deadline = Date.now() + 10_000;
        await tauriPage.click('.btn-primary');

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

    test('dev daemon configure writes hook entries to settings.json', async ({ tauriPage }) => {
        const before = readSettings();
        const hadDevHookBefore = devHookRegistered(before);

        const resp = await fetch(`${DAEMON_URL}/api/assistants/configure`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ acps: ['claude-code'] }),
        });
        expect(resp.status).toBe(200);

        await tauriPage.evaluate(`new Promise(r => setTimeout(r, 400))`);

        const after = readSettings();
        expect(after).not.toBeNull();
        expect(devHookRegistered(after), 'sensei-hook-dev.ts registered for core events').toBe(true);

        // Cleanup: remove if not present before
        if (!hadDevHookBefore) {
            await fetch(`${DAEMON_URL}/api/assistants/remove`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ acps: ['claude-code'] }),
            });
            await tauriPage.evaluate(`new Promise(r => setTimeout(r, 300))`);
            expect(devHookRegistered(readSettings()), 'entries removed after uninstall').toBe(false);
        }
    });

    test('configure is idempotent — no duplicate entries', async ({ tauriPage }) => {
        // Call configure twice
        for (let i = 0; i < 2; i++) {
            await fetch(`${DAEMON_URL}/api/assistants/configure`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ acps: ['claude-code'] }),
            });
        }
        await tauriPage.evaluate(`new Promise(r => setTimeout(r, 400))`);

        const settings = readSettings();
        const devEntries = hookEntries(settings, 'SessionStart').filter(cmd => cmd === DEV_HOOK);
        expect(devEntries).toHaveLength(1);

        // Cleanup
        await fetch(`${DAEMON_URL}/api/assistants/remove`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ acps: ['claude-code'] }),
        });
    });
});
