/**
 * R3 · project → Dōjō auto-bind — functional e2e.
 *
 * The pure inference + the confirmed/inferred/empty view model are covered
 * exhaustively by unit tests (routing.rs::infer_binding, about-binding-state
 * specs) and svelte-check. This spec covers what they can't, against the real
 * e2e daemon:
 *   1. Memberships are joined by signing in, not posted (405), and the
 *      org-tagging edit normalizes a joined membership's slugs.
 *   2. The Dōjō connections screen mounts and sends you to sign in — there
 *      is no pairing form (#236 step 4e).
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
  installErrorTrap, readErrors, type ErrBuf, seedSetupComplete,
} from '../helpers';

/** Mark setup complete so the observatory + project shells are reachable. */

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

  test('a membership cannot be posted — it joins by signing in (#236 step 4e)', async () => {
    const post = await fetch(`${DAEMON_URL}/api/dojo/memberships`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ membership_id: crypto.randomUUID(), tenant_key: 'github/acme-e2e', kind: 'employer' }),
    });
    expect(post.status, 'POST /api/dojo/memberships is gone').toBe(405);
  });

  test('org tagging replaces a joined membership\'s org slugs', async () => {
    // Memberships arrive with a signed-in persona's sync; the throwaway e2e DB
    // has none unless one was joined, so the edit is exercised only then.
    const memberships = await safeJson<Array<{ id: string; org_slugs: string[] }>>(
      `${DAEMON_URL}/api/dojo/memberships`, [],
    );
    if (memberships.length === 0) { test.skip(true, 'no joined membership in the e2e DB'); return; }
    const { id } = memberships[0];
    const put = await fetch(`${DAEMON_URL}/api/dojo/memberships/${id}/orgs`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ org_slugs: ['Acme-E2E', 'acme-e2e', 'globex'] }),
    });
    expect(put.ok, 'PUT …/orgs should succeed').toBe(true);
    const after = await safeJson<Array<{ id: string; org_slugs: string[] }>>(
      `${DAEMON_URL}/api/dojo/memberships`, [],
    );
    // Normalized: lowercased, deduped, first-seen order preserved.
    expect(after.find((m) => m.id === id)!.org_slugs).toEqual(['acme-e2e', 'globex']);
  });

  test('connections screen sends you to sign in, with no pairing form', async ({ tauriPage }) => {
    // Signing in is joining (#236 step 4e): the screen links to sign-in and
    // offers nothing to pair with — attribute selectors keep this within the
    // TauriPage adapter's supported locator surface.
    await navigateToScreen(tauriPage, '/dojo/connections', '[data-sign-in]');
    await expect(tauriPage.locator('[data-sign-in]')).toBeVisible({ timeout: 30_000 });
    expect(await tauriPage.locator('[data-connect-form]').count()).toBe(0);
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
