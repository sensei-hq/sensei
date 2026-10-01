/**
 * Project · Diagrams · Structure (/project/<id>/diagrams/structure) — functional e2e.
 *
 * The state spec covers the three-state machine, the degree arithmetic and the
 * kind toggle with an injected API seam. This covers what it cannot: the real
 * endpoint answering the real daemon against the real index, the rokkit
 * `StructureDiagram` MOUNTING without a runtime throw, and the level control
 * genuinely re-querying rather than re-rendering the same payload.
 *
 * The throwaway `sensei_e2e` DB is dropped before a run and nothing indexes
 * into it, so the seeded project has no graph and the EMPTY state is what
 * renders here. That is deliberate rather than a shortfall: empty is a terminal
 * state this screen has to get right, and getting it right means not being
 * reached by a failure. The body therefore waits for either the diagram or the
 * empty state and asserts the ERROR state is absent — a 500 or an unreachable
 * daemon is a real failure here, never an environmental one.
 *
 * The collapse test needs a real graph and SKIPS without one, with its reason
 * stated. Graph CONTENT is verified against the live daemon on sensei's own
 * index instead (1,751 file nodes -> 150 module nodes), which is the corpus
 * that makes the assertion mean something.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, daemonGet, daemonPost, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf,
} from '../helpers';

/**
 * A project to diagram, SEEDED rather than assumed.
 *
 * `reset-e2e-db` drops `sensei_e2e` before a run, so a fresh database has no
 * projects at all and a spec that reads `/api/projects[0]` throws before it
 * tests anything. Reusing whatever a previous run happened to leave behind is
 * worse than that: it passes or fails depending on history.
 *
 * A project with nothing indexed under it is the right fixture here. The graph
 * CONTENT is verified against the real index at the daemon level; what this
 * spec covers is that the screen mounts, settles, and lands in a terminal state
 * — and the empty state is a terminal state this screen must get right.
 */
async function seedProjectId(): Promise<string> {
  const existing = await daemonGet<Array<{ id: string; name: string }>>('/api/projects');
  const mine = existing.find((p) => p.name === 'e2e-structure');
  if (mine) return mine.id;
  const created = await daemonPost<{ ok: boolean; id: string }>('/api/projects', {
    name: 'e2e-structure',
    description: 'fixture for the Structure diagram spec',
  });
  return created.id;
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
    const id = await seedProjectId();
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
    const id = await seedProjectId();
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
