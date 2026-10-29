/**
 * Workspace · Diagrams · Schema — functional e2e (#221).
 *
 * The state spec covers filtering, the cut counts and the fallbacks against an
 * injected API. This covers what it cannot: the real endpoint answering through
 * the real app, and the screen's terminal state rendering in words.
 *
 * The throwaway `sensei_e2e` DB indexes nothing, so the workspace here describes
 * no database. That is the state most likely to lie: it must read as "nothing
 * described", never as a failure and never as an empty canvas. The picture over
 * real data is the inspect harness's job (`tests-inspect/schema-inspect.spec.ts`).
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, daemonPost,
  installErrorTrap, readErrors, type ErrBuf, seedSetupComplete,
  seedWorkspace,
} from '../helpers';

const seedWorkspaceId = (): Promise<string> => seedWorkspace('e2e-schema', 'fixture for the Schema diagram spec');

function expectNoRuntimeErrors(errs: ErrBuf, where: string): void {
  expect(errs.error, `${where}: uncaught errors`).toEqual([]);
  expect(errs.rejection, `${where}: unhandled rejections`).toEqual([]);
  expect(errs.console, `${where}: console.error output`).toEqual([]);
}

test.describe('Workspace · Diagrams · Schema', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(240_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('a workspace with no described database says so, and is not a failure', async ({ tauriPage }) => {
    const id = await seedWorkspaceId();
    await navigateToScreen(
      tauriPage,
      `/workspace/${id}/diagrams/schema`,
      '[data-testid="schema-screen"]',
    );
    await tauriPage.waitForSelector(
      '[data-testid="schema-none"], [data-testid="schema-error"], [data-testid="schema-diagram"]',
      60_000,
    );

    expect(
      await tauriPage.count('[data-testid="schema-error"]'),
      'the schema endpoint answered with an error, or could not be reached',
    ).toBe(0);
    expect(await tauriPage.count('[data-testid="schema-none"]')).toBe(1);
    expect(await tauriPage.count('[data-testid="schema-diagram"]')).toBe(0);

    expectNoRuntimeErrors(await readErrors(tauriPage), 'schema mount');
  });
});
