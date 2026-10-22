/**
 * Workspace · Diagrams · World — functional e2e.
 *
 * The state spec covers the view machine, the drill path, the "largest inside"
 * list and the absent-measure wording against an injected API seam. This covers
 * what it cannot: the real endpoint answering, and `@rokkit/graph`'s `Graph`
 * MOUNTING under the `world` layout without a runtime throw.
 *
 * That last one is the reason this file exists. The world layout is driven
 * through a `GraphState` rather than props — `shadeBy`, `focusPath` and `levels`
 * live on `GraphStateConfig` — so a node shape the layout rejects fails at mount
 * and nowhere earlier. `svelte-check` passes on it, the unit tests pass on it,
 * and the canvas is blank.
 *
 * The throwaway `sensei_e2e` DB is dropped before a run and nothing indexes into
 * it, so the EMPTY state is what renders here. That is a terminal state this
 * screen must get right — and on a picture whose whole subject is how much there
 * is, an empty canvas with no words is the worst of the three.
 */

import { test, expect, type TauriPageLike } from '../fixtures';
import {
  navigateTo, navigateToScreen, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf, seedSetupComplete,
  seedWorkspace,
} from '../helpers';

const seedWorkspaceId = (): Promise<string> => seedWorkspace('e2e-world', 'fixture for the World diagram spec');


function control(bar: string, item: string): string {
  return `[data-toolbar][aria-label="${bar}"] button[data-toolbar-item][aria-label="${item}"]`;
}

function expectNoRuntimeErrors(errs: ErrBuf, where: string): void {
  expect(errs.error, `${where}: uncaught errors`).toEqual([]);
  expect(errs.rejection, `${where}: unhandled rejections`).toEqual([]);
  expect(errs.console, `${where}: console.error output`).toEqual([]);
}

async function settle(tauriPage: TauriPageLike): Promise<void> {
  await tauriPage.waitForSelector(
    '[data-testid="world-diagram"], [data-testid="world-empty"], [data-testid="world-error"]',
    120_000,
  );
}

test.describe('Workspace · Diagrams · World', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(240_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('mounts, settles out of loading, and never shows the error state', async ({ tauriPage }) => {
    const id = await seedWorkspaceId();
    await navigateToScreen(tauriPage, `/workspace/${id}/diagrams/world`, '[data-testid="world-screen"]');

    // Chrome renders in every state, and the groupings are the three that are
    // FACTS. A fourth would mean a docs ring had been added without the data
    // behind it (#246).
    expect(await tauriPage.count(control('Group first by', 'Workspace'))).toBe(1);
    expect(await tauriPage.count(control('Group first by', 'Repository'))).toBe(1);
    expect(await tauriPage.count(control('Group first by', 'Code · tests'))).toBe(1);
    expect(
      await tauriPage.count('[data-toolbar][aria-label="Group first by"] button[data-toolbar-item]'),
      'three groupings, because documentation is not a declaration (#246)',
    ).toBe(3);

    // `Nothing` is the default shade and a choice: the picture's first job is
    // size, and colouring before anyone asked turns a glance into a reading.
    expect(await tauriPage.getAttribute(control('Shade by', 'Nothing'), 'data-active')).toBe('true');

    await settle(tauriPage);

    expect(
      await tauriPage.count('[data-testid="world-error"]'),
      'the daemon answered with an error, or could not be reached',
    ).toBe(0);
    expect(await tauriPage.count('[data-testid="world-loading"]')).toBe(0);

    const drawn = await tauriPage.count('[data-testid="world-diagram"]');
    const empty = await tauriPage.count('[data-testid="world-empty"]');
    expect(drawn + empty, 'exactly one terminal state renders').toBe(1);

    // WHENEVER THE PICTURE IS DRAWN it says it is wider than the workspace. A view
    // of every workspace silently reads as a view of yours, which is the one
    // misreading this screen can produce.
    if (drawn > 0) {
      const viewing = await tauriPage.textContent('[data-testid="world-viewing"]');
      expect(viewing).toContain('not only this workspace');
      expect(await tauriPage.count('[data-testid="world-totals"]')).toBe(1);
      expect(await tauriPage.count('[data-testid="world-inview"]')).toBe(1);
      const label = await tauriPage.textContent('[data-testid="world-inview-label"]');
      expect(label, 'the root is a named place, not a blank panel').toContain('All indexed code');
    }

    expectNoRuntimeErrors(await readErrors(tauriPage), 'world mount');
  });

  test('changing the grouping re-queries without erroring', async ({ tauriPage }) => {
    const id = await seedWorkspaceId();
    await navigateToScreen(tauriPage, `/workspace/${id}/diagrams/world`, '[data-testid="world-screen"]');
    await settle(tauriPage);

    await tauriPage.locator(control('Group first by', 'Repository')).click();
    await settle(tauriPage);

    expect(await tauriPage.count('[data-testid="world-error"]')).toBe(0);
    expect(
      await tauriPage.getAttribute(control('Group first by', 'Repository'), 'data-active'),
    ).toBe('true');

    expectNoRuntimeErrors(await readErrors(tauriPage), 'world grouping change');
  });
});
