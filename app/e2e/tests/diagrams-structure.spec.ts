/**
 * Project · Diagrams · Structure (/project/<id>/diagrams/structure) — functional e2e.
 *
 * The state spec covers the three-state machine, the degree arithmetic and the
 * kind toggle with an injected API seam. This covers what it cannot: the real
 * endpoint answering the real daemon against the real index, the rokkit
 * `StructureDiagram` MOUNTING without a runtime throw, and the level control
 * genuinely re-querying rather than re-rendering the same payload.
 *
 * Whether the throwaway `sensei_e2e` DB has an indexed graph depends on prior
 * runs, so the body waits for EITHER the diagram OR the honest empty state —
 * exactly one always renders, and both prove the fetch settled without
 * collapsing into the other. What is NEVER acceptable is the error state, which
 * is asserted absent: a 500 or an unreachable daemon is a real failure here, not
 * an environmental one.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, daemonGet, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf,
} from '../helpers';

async function anyProjectId(): Promise<string> {
  const projects = await daemonGet<Array<{ id: string; name: string }>>('/api/projects');
  if (!projects.length) throw new Error('e2e daemon has no projects to diagram');
  return projects[0].id;
}

async function seedSetupComplete(tauriPage: any): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function() {
      try { localStorage.setItem('sensei:setup-complete', '1'); } catch (e) { /* shim */ }
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

function expectNoRuntimeErrors(errs: ErrBuf, where: string): void {
  expect(errs.error, `${where}: uncaught errors`).toEqual([]);
  expect(errs.rejection, `${where}: unhandled rejections`).toEqual([]);
  expect(errs.console, `${where}: console.error output`).toEqual([]);
}

/** Wait past the loading state to whichever terminal state the data produces. */
async function settle(tauriPage: any): Promise<void> {
  await tauriPage.waitForSelector(
    '[data-testid="structure-diagram"], [data-testid="structure-empty"], [data-testid="structure-error"]',
    30_000,
  );
}

test.describe('Project · Diagrams · Structure', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('mounts, settles out of loading, and never shows the error state', async ({ tauriPage }) => {
    const id = await anyProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/structure`,
      '[data-testid="structure-screen"]',
    );

    // Chrome renders in every state — the controls are not gated on data.
    expect(await tauriPage.count('[data-testid="structure-level-file"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="structure-level-module"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="structure-kind-calls"]')).toBe(1);

    await settle(tauriPage);

    // The three states are mutually exclusive, and error is a REAL failure.
    expect(
      await tauriPage.count('[data-testid="structure-error"]'),
      'the daemon answered with an error, or could not be reached',
    ).toBe(0);
    expect(await tauriPage.count('[data-testid="structure-loading"]')).toBe(0);

    const drawn = await tauriPage.count('[data-testid="structure-diagram"]');
    const empty = await tauriPage.count('[data-testid="structure-empty"]');
    expect(drawn + empty, 'exactly one terminal state renders').toBe(1);

    // Whenever the graph is drawn, what it is NOT showing is stated beside it.
    if (drawn > 0) {
      expect(await tauriPage.count('[data-testid="structure-coverage"]')).toBe(1);
      const line = await tauriPage.textContent('[data-testid="structure-coverage"]');
      expect(line).toContain('unplaced (not shown)');
    }

    expectNoRuntimeErrors(await readErrors(tauriPage), 'structure mount');
  });

  test('switching to module level re-queries and collapses the graph', async ({ tauriPage }) => {
    const id = await anyProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/structure`,
      '[data-testid="structure-screen"]',
    );
    await settle(tauriPage);

    // Only meaningful when there is a graph to collapse.
    test.skip(
      (await tauriPage.count('[data-testid="structure-diagram"]')) === 0,
      'no indexed graph in this e2e database',
    );

    const fileLine = (await tauriPage.textContent('[data-testid="structure-coverage"]')) ?? '';
    const fileNodes = Number(fileLine.match(/^(\d+) nodes/)?.[1] ?? '0');
    expect(fileNodes).toBeGreaterThan(0);

    await tauriPage.locator('[data-testid="structure-level-module"]').click();
    await settle(tauriPage);

    expect(await tauriPage.count('[data-testid="structure-error"]')).toBe(0);
    expect(
      await tauriPage.getAttribute('[data-testid="structure-level-module"]', 'aria-pressed'),
    ).toBe('true');

    const moduleLine = (await tauriPage.textContent('[data-testid="structure-coverage"]')) ?? '';
    const moduleNodes = Number(moduleLine.match(/^(\d+) nodes/)?.[1] ?? '0');

    // A rollup that does not reduce is not a rollup. This is the property that
    // made `module` mean the module's TOP segment rather than the whole path:
    // the whole path collapses 1,751 files to 1,446 on sensei's own corpus,
    // because that segment is per-file across most of the tree.
    expect(moduleNodes).toBeGreaterThan(0);
    expect(moduleNodes, 'module level must collapse the file graph').toBeLessThan(fileNodes);

    expectNoRuntimeErrors(await readErrors(tauriPage), 'structure level switch');
  });
});
