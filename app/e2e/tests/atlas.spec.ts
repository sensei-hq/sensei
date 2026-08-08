/**
 * Project · Atlas (/project/<id>/atlas) — functional e2e.
 *
 * The atlas is a per-project screen now (all graphs are at project level). The
 * unit tests + svelte-check cover the pure graph shaping. This spec covers what
 * they can't: the loader firing the real graph reads against the e2e daemon
 * SCOPED to a project's repo, the screen MOUNTING without a runtime throw, the
 * d3-force canvas NOT crashing, and the granularity toggle switching modes.
 *
 * Whether the throwaway `sensei_e2e` DB has an indexed graph for the project's
 * scope depends on prior runs, so the test waits for EITHER the d3-force canvas
 * OR the honest empty-state affordance (exactly one always renders). Both paths
 * assert NO uncaught error / rejection / console.error.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, waitForDom, daemonGet, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf,
} from '../helpers';

/** A project + the repo name its atlas scopes to (the project's primary git root,
 *  else the project name). Intake/atlas are project-scoped, so we need a real id. */
async function projectScope(): Promise<{ id: string; repo: string }> {
  const projects = await daemonGet<Array<{ id: string; name: string; folders?: Array<{ kind: string; name: string }> }>>('/api/projects');
  for (const p of projects) {
    const git = (p.folders ?? []).find((f) => f.kind === 'git');
    if (git) return { id: p.id, repo: git.name };
  }
  if (projects.length) return { id: projects[0].id, repo: projects[0].name };
  throw new Error('e2e daemon has no projects to scope the atlas to');
}

/** Mark setup complete so the app shell is reachable. */
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

test.describe('Project · Atlas', () => {
  test.beforeEach(async ({ tauriPage }) => {
    // Headroom for the retry-navigate budget: cold health-bootstrap + setup
    // reconcile for the FIRST gated screen can take ~50s on a loaded box.
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA to a setup-exempt route
    await installErrorTrap(tauriPage);
  });

  test('mounts and renders chrome + graph/empty affordance without a runtime throw', async ({ tauriPage }) => {
    const { id, repo } = await projectScope();
    await navigateToScreen(tauriPage, `/project/${id}/atlas`, '[data-screen="atlas"]');

    // Screen mounted — the loader completed the graph reads (incl. the
    // detectCommunities POST) against the real daemon without throwing, scoped to
    // THIS project's repo (proves the payload assembled with the project scope).
    expect(await tauriPage.getAttribute('[data-screen="atlas"]', 'data-atlas-repo')).toBe(repo);

    // Chrome that renders in both states: the scope select + the granularity
    // segmented control (communities / symbols).
    expect(await tauriPage.count('[data-atlas-scope]')).toBe(1);
    expect(await tauriPage.count('[data-atlas-level="communities"]')).toBe(1);
    expect(await tauriPage.count('[data-atlas-level="symbols"]')).toBe(1);

    // The body renders EXACTLY one of two mutually-exclusive states — the d3-force
    // canvas OR the honest empty-state affordance — depending on whether the e2e
    // DB has an indexed graph for this scope. Both prove the loader completed +
    // the body mounted with no throw.
    await tauriPage.waitForSelector('[data-component="atlas-canvas"], [data-empty]', 20_000);
    const canvas = await tauriPage.count('[data-component="atlas-canvas"]');
    const empty = await tauriPage.count('[data-empty]');
    expect(canvas + empty).toBeGreaterThan(0);
    if (canvas > 0) {
      expect(await tauriPage.count('[data-component="atlas-legend"]')).toBeGreaterThan(0);
    }

    expectNoRuntimeErrors(await readErrors(tauriPage), 'atlas mount');
  });

  test('toggling granularity (communities ↔ symbols) does not throw', async ({ tauriPage }) => {
    const { id } = await projectScope();
    await navigateToScreen(tauriPage, `/project/${id}/atlas`, '[data-screen="atlas"]');

    // Opening state: communities is the default for an unindexed / large scope.
    const opensOn = await tauriPage.getAttribute('[data-atlas-level="communities"]', 'aria-pressed');

    // Flip to symbols — must re-render (and, when data exists, re-run the force
    // layout) without a throw.
    await tauriPage.locator('[data-atlas-level="symbols"]').click();
    await waitForDom(
      tauriPage,
      `document.querySelector('[data-atlas-level="symbols"]').getAttribute('aria-pressed') === 'true'`,
    );

    // Flip back to communities.
    await tauriPage.locator('[data-atlas-level="communities"]').click();
    await waitForDom(
      tauriPage,
      `document.querySelector('[data-atlas-level="communities"]').getAttribute('aria-pressed') === 'true'`,
    );

    // The screen is still mounted (no crash unmounted it) and the trap is clean.
    expect(await tauriPage.count('[data-screen="atlas"]')).toBe(1);
    expect(opensOn === 'true' || opensOn === 'false').toBe(true);
    expectNoRuntimeErrors(await readErrors(tauriPage), 'atlas toggle');
  });
});
