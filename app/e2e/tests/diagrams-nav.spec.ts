/**
 * Workspace · Diagrams — the sidebar's Diagrams subsection (M16/M17 mockup, user
 * 2026-10-29): one entry per diagram, each its own route on one shared Code map.
 *
 * The unit specs cover the list, the route's 404s and the lens switch against an
 * injected port. This covers the real window: the subsection in the rail, the
 * default redirect, and a click on an entry changing the lens and the current
 * entry together, with nothing thrown on the way.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo,
  installErrorTrap, readErrors, seedSetupComplete, seedWorkspace,
} from '../helpers';

const DIAGRAMS = ['Structure', 'Wheel', 'Zones', 'Dependency matrix', 'Cycles', 'Hidden coupling', 'Complexity', 'Database · ER'];

const current = (tauriPage: { evaluate: (js: string) => Promise<unknown> }) =>
  tauriPage.evaluate(
    `document.querySelector('[data-component="workspace-sidebar"] [data-list-item][data-active="true"] [data-item-label]')?.textContent?.trim() ?? null`,
  );

test.describe('Workspace · Diagrams nav', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(240_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('the sidebar lists each diagram, and an entry opens its lens', async ({ tauriPage }) => {
    const id = await seedWorkspace('e2e-diagrams-nav', 'fixture for the Diagrams nav spec');
    // The section's own URL redirects to its first diagram.
    await navigateTo(tauriPage, `/workspace/${id}/diagrams`);
    await tauriPage.waitForSelector('[data-testid="codemap-screen"][data-lens="structure"]', 60_000);
    expect(await tauriPage.evaluate('location.pathname')).toBe(`/workspace/${id}/diagrams/structure`);

    const labels = (await tauriPage.evaluate(
      `[...document.querySelectorAll('[data-component="workspace-sidebar"] a[href*="/diagrams/"] [data-item-label]')].map((e) => e.textContent.trim())`,
    )) as string[];
    expect(labels).toEqual(DIAGRAMS);
    expect(await current(tauriPage)).toBe('Structure');

    await tauriPage.locator(`[data-component="workspace-sidebar"] a[href="/workspace/${id}/diagrams/wheel"]`).click();
    await tauriPage.waitForSelector('[data-testid="codemap-screen"][data-lens="wheel"]', 30_000);
    expect(await current(tauriPage)).toBe('Wheel');
    expect(await tauriPage.count('[data-testid="diagrams-tab-structure"]'), 'no tab row above the diagram').toBe(0);

    const errs = await readErrors(tauriPage);
    expect([errs.error, errs.rejection, errs.console]).toEqual([[], [], []]);
  });
});
