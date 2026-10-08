/**
 * Project · Diagrams · Neighbourhood — functional e2e.
 *
 * The state spec covers the view machine, the coverage wording, re-centring and
 * the stale-response guard against an injected API seam. This covers what it
 * cannot: the real endpoint and the real search answering, and the screen's
 * terminal states rendering with words.
 *
 * The throwaway `sensei_e2e` DB is dropped before a run and nothing indexes into
 * it, so there is no symbol to centre on here. What CAN be asserted is the part
 * most likely to lie: the screen opens on a question rather than a default
 * symbol, a search with no answer says so, and a focus the project does not
 * have is an error naming why — never a blank canvas.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, daemonGet, daemonPost, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf, seedSetupComplete,
} from '../helpers';

async function seedProjectId(): Promise<string> {
  const existing = await daemonGet<Array<{ id: string; name: string }>>('/api/projects');
  const mine = existing.find((p) => p.name === 'e2e-neighbourhood');
  if (mine) return mine.id;
  const created = await daemonPost<{ ok: boolean; id: string }>('/api/projects', {
    name: 'e2e-neighbourhood',
    description: 'fixture for the Neighbourhood diagram spec',
  });
  return created.id;
}


function expectNoRuntimeErrors(errs: ErrBuf, where: string): void {
  expect(errs.error, `${where}: uncaught errors`).toEqual([]);
  expect(errs.rejection, `${where}: unhandled rejections`).toEqual([]);
  expect(errs.console, `${where}: console.error output`).toEqual([]);
}

test.describe('Project · Diagrams · Neighbourhood', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(240_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('opens on a question, not a default symbol', async ({ tauriPage }) => {
    const id = await seedProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/neighbourhood`,
      '[data-testid="neighbourhood-screen"]',
    );

    expect(await tauriPage.count('[data-testid="neighbourhood-pick"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="neighbourhood-diagram"]')).toBe(0);
    expect(
      await tauriPage.count('[data-toolbar][aria-label="Depth"]'),
      'depth means nothing without a focus',
    ).toBe(0);
    expect(await tauriPage.count('[data-testid="diagrams-tab-neighbourhood"]')).toBe(1);

    expectNoRuntimeErrors(await readErrors(tauriPage), 'neighbourhood mount');
  });

  test('a search with no answer says so, and is not a failure', async ({ tauriPage }) => {
    const id = await seedProjectId();
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/neighbourhood`,
      '[data-testid="neighbourhood-screen"]',
    );

    await tauriPage.locator('[data-testid="neighbourhood-search"]').fill('zz_no_such_symbol');
    await tauriPage.waitForSelector(
      '[data-testid="neighbourhood-no-matches"], [data-testid="neighbourhood-search-error"]',
      60_000,
    );
    expect(
      await tauriPage.count('[data-testid="neighbourhood-search-error"]'),
      'the search endpoint answered with an error, or could not be reached',
    ).toBe(0);
    expect(await tauriPage.count('[data-testid="neighbourhood-no-matches"]')).toBe(1);

    expectNoRuntimeErrors(await readErrors(tauriPage), 'neighbourhood search');
  });

  test('a focus this project does not have is an error that says why', async ({ tauriPage }) => {
    const id = await seedProjectId();
    const stranger = '00000000-0000-4000-8000-000000000000';
    await navigateToScreen(
      tauriPage,
      `/project/${id}/diagrams/neighbourhood?focus=${stranger}`,
      '[data-testid="neighbourhood-screen"]',
    );
    await tauriPage.waitForSelector('[data-testid="neighbourhood-error"]', 60_000);

    const text = await tauriPage.textContent('[data-testid="neighbourhood-error"]');
    expect(text).toContain('Not a symbol of this project');
    expect(await tauriPage.count('[data-testid="neighbourhood-diagram"]')).toBe(0);
  });
});
