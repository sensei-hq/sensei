/**
 * R3 · project → Dōjō auto-bind — functional e2e.
 *
 * The pure inference + the confirmed/inferred/empty view model are covered
 * exhaustively by unit tests (routing.rs::infer_binding, about-binding-state
 * specs) and svelte-check. This spec covers what they can't, against the real
 * e2e daemon:
 *   1. org_slugs round-trips end-to-end — POST /api/dojo/memberships with
 *      org_slugs, GET returns them normalized (proves the org_slugs DDL column
 *      + handler work on a live daemon, not just sensei_test).
 *   2. The Dōjō connections screen mounts and its connect form exposes the
 *      org-tagging input (data-testid="connect-org-slugs").
 *   3. The project About "Bindings" section mounts without a runtime throw and
 *      renders exactly one valid state (confirmed | inferred | empty) — proves
 *      the loader fires GET …/dojo-suggestion + the membership list and the
 *      section renders. The inferred→confirm click-flow needs a project whose
 *      git remote owner matches a membership's org_slugs; the throwaway e2e DB
 *      has no scanned git projects, so that path is unit-tested, not e2e'd.
 *
 * Requires the org_slugs column in sensei_e2e — run with
 * `SENSEI_DDL_DIR=$(pwd)/database` until a release bundles the DDL.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf,
} from '../helpers';

/** Mark setup complete so the observatory + project shells are reachable. */
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

async function safeJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(url);
    if (!res.ok) return fallback;
    const text = await res.text();
    return text ? (JSON.parse(text) as T) : fallback;
  } catch {
    return fallback;
  }
}

test.describe('R3 · Dōjō auto-bind', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // setup-exempt route to reset the SPA
    await installErrorTrap(tauriPage);
  });

  test('org_slugs round-trips through the daemon (POST → GET normalized)', async () => {
    // A membership id the daemon will accept as the PK. org_slugs given with
    // mixed case + a dupe; the daemon normalizes to lowercased + deduped.
    const membershipId = crypto.randomUUID();
    const body = {
      membership_id: membershipId,
      tenant_key: 'github/acme-e2e',
      kind: 'employer',
      org_slugs: ['Acme-E2E', 'acme-e2e', 'acme-labs'],
      credential: 'e2e-device-token',
    };
    const post = await fetch(`${DAEMON_URL}/api/dojo/memberships`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    expect(post.ok, 'POST /api/dojo/memberships should succeed').toBe(true);

    const memberships = await safeJson<Array<{ id: string; org_slugs: string[] }>>(
      `${DAEMON_URL}/api/dojo/memberships`, [],
    );
    const created = memberships.find((m) => m.id === membershipId);
    expect(created, 'created membership is listed').toBeTruthy();
    // Normalized: lowercased, deduped, first-seen order preserved.
    expect(created!.org_slugs).toEqual(['acme-e2e', 'acme-labs']);

    // The org-tagging edit endpoint replaces the set.
    const put = await fetch(`${DAEMON_URL}/api/dojo/memberships/${membershipId}/orgs`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ org_slugs: ['acme-e2e', 'globex'] }),
    });
    expect(put.ok, 'PUT …/orgs should succeed').toBe(true);
    const after = await safeJson<Array<{ id: string; org_slugs: string[] }>>(
      `${DAEMON_URL}/api/dojo/memberships`, [],
    );
    expect(after.find((m) => m.id === membershipId)!.org_slugs).toEqual(['acme-e2e', 'globex']);
  });

  test('connections screen mounts with the org-tagging input', async ({ tauriPage }) => {
    // Wait for the (always-present) connect toggle, then open the disclosure
    // form and assert the org-tagging input renders — attribute selectors keep
    // this within the TauriPage adapter's supported locator surface.
    await navigateToScreen(tauriPage, '/dojo/connections', '[data-connect-toggle]');
    await tauriPage.locator('[data-connect-toggle]').click();
    await expect(tauriPage.locator('[data-testid="connect-org-slugs"]')).toBeVisible({ timeout: 30_000 });
    expectNoRuntimeErrors(await readErrors(tauriPage), 'connections');
  });

  test('project About Bindings section mounts in a valid state', async ({ tauriPage }) => {
    const projects = await safeJson<Array<{ id: string }>>(`${DAEMON_URL}/api/projects`, []);
    if (projects.length === 0) { test.skip(true, 'no projects registered in e2e DB'); return; }
    const id = projects[0].id;

    await navigateToScreen(tauriPage, `/project/${id}/about`, '[data-testid="project-bindings"]');
    const section = tauriPage.locator('[data-testid="project-bindings"]');
    await expect(section).toBeVisible({ timeout: 30_000 });

    // Exactly one valid state renders: a confirmed/inferred chip, or the empty
    // hint. With no scanned git-owner project + membership match, empty is
    // expected — but assert the section is coherent either way.
    const chip = section.locator('[data-binding-state]');
    const emptyHint = section.getByText(/stay local/i);
    const hasChip = await chip.count() > 0;
    const hasEmpty = await emptyHint.count() > 0;
    expect(hasChip || hasEmpty, 'Bindings renders a chip or the empty hint').toBe(true);
    expectNoRuntimeErrors(await readErrors(tauriPage), 'about-bindings');
  });
});
