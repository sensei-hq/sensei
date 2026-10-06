/**
 * Project · Diagrams · Layers and Cycles — functional e2e.
 *
 * The state spec covers the three-state machine, the break filter, the cycle
 * ordering and the cut sentence against an injected API seam. This covers what
 * it cannot: the real endpoint answering the real daemon, the rokkit
 * `LayersDiagram` MOUNTING without a runtime throw, and the sub-nav actually
 * routing between the three views.
 *
 * The throwaway `sensei_e2e` DB is dropped before a run and nothing indexes into
 * it, so the seeded project has no graph and the EMPTY state is what renders
 * here. That is deliberate: empty is a terminal state these screens must get
 * right, and getting it right means not being reached by a failure. Each body
 * waits for a terminal state and asserts the ERROR state is absent — a 500 or an
 * unreachable daemon is a real failure here, never an environmental one.
 *
 * CONTENT is verified against the live daemon on sensei's own index instead, at
 * the endpoint level, which is the corpus that makes it mean something: 154
 * units over 4 layers, 25 climbs against 15 skips, and one 22-member cycle whose
 * weakest link is `senseid/adapters -> senseid/tasks`.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, daemonGet, daemonPost, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf,
} from '../helpers';

async function seedProjectId(): Promise<string> {
  const existing = await daemonGet<Array<{ id: string; name: string }>>('/api/projects');
  const mine = existing.find((p) => p.name === 'e2e-layers');
  if (mine) return mine.id;
  const created = await daemonPost<{ ok: boolean; id: string }>('/api/projects', {
    name: 'e2e-layers',
    description: 'fixture for the Layers and Cycles diagram specs',
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

async function settleLayers(tauriPage: any): Promise<void> {
  await tauriPage.waitForSelector(
    '[data-testid="layers-diagram"], [data-testid="layers-empty"], [data-testid="layers-error"]',
    120_000,
  );
}

async function settleCycles(tauriPage: any): Promise<void> {
  await tauriPage.waitForSelector(
    '[data-testid="cycles-list"], [data-testid="cycles-none"], [data-testid="cycles-empty"], [data-testid="cycles-error"]',
    120_000,
  );
}

test.describe('Project · Diagrams · Layers', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(240_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('mounts, settles out of loading, and never shows the error state', async ({ tauriPage }) => {
    const id = await seedProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/layers`,
      '[data-testid="layers-screen"]',
    );

    // Chrome renders in every state — the controls are not gated on data.
    expect(await tauriPage.count('[data-testid="layers-level-module"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="layers-level-file"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="layers-kind-calls"]')).toBe(1);

    await settleLayers(tauriPage);

    expect(
      await tauriPage.count('[data-testid="layers-error"]'),
      'the daemon answered with an error, or could not be reached',
    ).toBe(0);
    expect(await tauriPage.count('[data-testid="layers-loading"]')).toBe(0);

    const drawn = await tauriPage.count('[data-testid="layers-diagram"]');
    const empty = await tauriPage.count('[data-testid="layers-empty"]');
    expect(drawn + empty, 'exactly one terminal state renders').toBe(1);

    // Whenever the picture is drawn, all three readings sit beside it: what the
    // layering IS, what it found, and what the picture leaves out. The middle
    // one is the reason this screen cannot ship as a bare canvas — an empty
    // violations list with no caption reads as a clean architecture.
    if (drawn > 0) {
      expect(await tauriPage.count('[data-testid="layers-source"]')).toBe(1);
      expect(await tauriPage.count('[data-testid="layers-breaks"]')).toBe(1);
      expect(await tauriPage.count('[data-testid="layers-coverage"]')).toBe(1);
      const source = await tauriPage.textContent('[data-testid="layers-source"]');
      expect(source, 'the screen must say the layering was measured, not declared').toContain(
        'measured',
      );
      const coverage = await tauriPage.textContent('[data-testid="layers-coverage"]');
      expect(coverage).toContain('owns no file');
    }

    expectNoRuntimeErrors(await readErrors(tauriPage), 'layers mount');
  });

  test('the sub-nav routes between the three views', async ({ tauriPage }) => {
    const id = await seedProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/layers`,
      '[data-testid="layers-screen"]',
    );

    expect(await tauriPage.count('[data-testid="diagrams-tab-structure"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="diagrams-tab-layers"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="diagrams-tab-cycles"]')).toBe(1);
    expect(
      await tauriPage.getAttribute('[data-testid="diagrams-tab-layers"]', 'aria-current'),
      'the tab for the screen you are on is the current one',
    ).toBe('page');

    await tauriPage.locator('[data-testid="diagrams-tab-cycles"]').click();
    await tauriPage.waitForSelector('[data-testid="cycles-screen"]', 30_000);
    expect(await tauriPage.getAttribute('[data-testid="diagrams-tab-cycles"]', 'aria-current')).toBe(
      'page',
    );

    expectNoRuntimeErrors(await readErrors(tauriPage), 'diagrams sub-nav');
  });
});

test.describe('Project · Diagrams · Cycles', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(240_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('settles into a terminal state and says so when there are no cycles', async ({
    tauriPage,
  }) => {
    const id = await seedProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/cycles`,
      '[data-testid="cycles-screen"]',
    );

    await settleCycles(tauriPage);

    expect(await tauriPage.count('[data-testid="cycles-error"]')).toBe(0);
    expect(await tauriPage.count('[data-testid="cycles-loading"]')).toBe(0);

    const listed = await tauriPage.count('[data-testid="cycles-list"]');
    const none = await tauriPage.count('[data-testid="cycles-none"]');
    const empty = await tauriPage.count('[data-testid="cycles-empty"]');
    expect(listed + none + empty, 'exactly one terminal state renders').toBe(1);

    // THE READING THIS SCREEN EXISTS FOR. "No cycles" and "nothing loaded" are
    // the same blank canvas, so the absence gets words — and a project that DID
    // load with no cycles must land on `cycles-none`, never on `cycles-empty`.
    if (none > 0) {
      const text = await tauriPage.textContent('[data-testid="cycles-none"]');
      expect(text).toContain('No unit depends on itself');
    }

    expectNoRuntimeErrors(await readErrors(tauriPage), 'cycles mount');
  });
});
