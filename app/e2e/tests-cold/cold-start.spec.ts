// Cold-start E2E spec — verifies the health page drives itself through
// the full check → resolve → land flow with NO test-driven navigation.
//
// Setup (globalSetup-cold.ts) has already:
//   • dropped sensei_dev
//   • stopped postgresql@17 + ollama
//   • launched Sensei.app without pre-starting senseid
//
// What we assert:
//   1. The window lands on /health *without* the test calling goto().
//   2. The ledger renders 5 component rows with badges showing "checking"
//      from the moment of mount (verifies the checking-by-default fix).
//   3. At least one component badge transitions out of "checking" within
//      ~5s of mount (verifies streaming probes — without it we'd wait
//      the full check phase before any DOM motion).
//   4. The page enters the resolve phase on its own (Hero copy changes).
//   5. The URL eventually transitions to /setup/welcome or / — proving
//      the auto-advance $effect fires when all components are green.

import { test, expect } from '../fixtures';

test.describe('Cold start', () => {
  test('mounts to /health with all components in checking', async ({ tauriPage }) => {
    // The page should appear on /health on its own. globalSetup-cold did
    // not pre-navigate, and there's no goto() in the spec.
    // tauri-playwright waitForURL is a string glob, not a regex.
    await tauriPage.waitForURL('/health', { timeout: 30_000 });

    // Ledger renders 5 component rows (postgres, ollama, sensei, database, daemon).
    // The package manager (homebrew) lives in the Hero, not the Ledger.
    await expect(tauriPage.locator('[data-row]')).toHaveCount(5, { timeout: 5_000 });

    // Every row badge says "checking" — the dcaefded fix. Before that,
    // they would all say "pending" with 55% opacity (visually blank).
    const badges = tauriPage.locator('[data-row] [data-badge]');
    await expect(badges).toHaveCount(5);
    for (let i = 0; i < 5; i++) {
      await expect(badges.nth(i)).toHaveText('checking', { timeout: 3_000 });
    }
  });

  test('component badges transition out of checking as probes return', async ({ tauriPage }) => {
    await tauriPage.waitForURL('/health', { timeout: 30_000 });

    // Within 10 s, at least one component should have flipped out of
    // 'checking' (postgres → failed, ollama → failed, etc.). Without
    // the 7b8c5e89 streaming fix + the 83dfd7b4 timeout fix, every
    // badge would stay "checking" for ~13 s before all flipping at once.
    const stillChecking = tauriPage.locator('[data-row] [data-badge]', { hasText: /^checking$/ });
    await expect(stillChecking).not.toHaveCount(5, { timeout: 10_000 });
  });

  test('enters resolve phase without test intervention', async ({ tauriPage }) => {
    await tauriPage.waitForURL('/health', { timeout: 30_000 });

    // Hero h1 changes copy per status. 'resolving' copy is
    // "Setting up your foundation." — see Header.svelte.
    await expect(tauriPage.locator('h1'))
      .toContainText(/Setting up/i, { timeout: 60_000 });
  });

  test('auto-advances off /health once all components are green', async ({ tauriPage }) => {
    await tauriPage.waitForURL('/health', { timeout: 30_000 });

    // Resolvers run brew install/start + dbd deploy + senseid start.
    // Total wall time on a warm dev box: ~30-90 s. Allow generous budget.
    // The $effect in (health)/health/+page.svelte calls goto('/') when
    // healthState.isOk. The reroute hook then sends us to /setup/welcome
    // because the cold setup dropped the DB, so setup-complete is false.
    await tauriPage.waitForURL('/setup/welcome', { timeout: 180_000 });
  });
});
