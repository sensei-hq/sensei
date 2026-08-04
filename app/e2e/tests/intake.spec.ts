/**
 * Intake — now a PER-PROJECT front door.
 *
 * A playbook run always happens IN a project (no cwd/repo/graph/rules scope for a
 * global run), so intake moved off the observatory rail into the project window.
 * This verifies /project/<id>/intake end-to-end in the real built Tauri app against
 * a fresh e2e daemon: the route renders, the project sidebar links to it, and the
 * freeform → recommend → confirm flow round-trips through the daemon (classify +
 * recommend + record) scoped to a real project.
 *
 * Seed-agnostic: the throwaway sensei_e2e DB has the playbook DDL but no imported
 * catalog/rules, so `recommend` returns a defaulted playbook (raw name, no
 * title/tone). The flow is identical — this asserts the plumbing, not the copy.
 */

import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, daemonGet, daemonPost, DAEMON_URL } from '../helpers';

/** Mark setup complete so the app shell is reachable (not rerouted to setup). */
async function seedSetupComplete(tauriPage: any): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function() {
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

async function pathname(tauriPage: any): Promise<string> {
  return (await tauriPage.evaluate(`window.location.pathname`)) as string;
}

/** First project in the e2e DB — intake is project-scoped, so we need a real id. */
async function firstProjectId(): Promise<string> {
  const projects = await daemonGet<Array<{ id: string }>>('/api/projects');
  if (!projects.length) throw new Error('e2e daemon has no projects to scope intake to');
  return projects[0].id;
}

test.describe('Intake — per-project front door', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target
  });

  test('direct navigation renders the project intake screen', async ({ tauriPage }) => {
    // Cold-boot health-ready can take ~50s on a fresh e2e DB; lift the per-test cap.
    test.setTimeout(180_000);
    const id = await firstProjectId();
    await navigateToScreen(tauriPage, `/project/${id}/intake`, '[data-testid="intake-input"]');
    expect(await pathname(tauriPage)).toBe(`/project/${id}/intake`);
    await expect(tauriPage.locator('[data-testid="intake-input"]')).toBeVisible();
    await expect(tauriPage.locator('[data-testid="intake-recommend"]')).toBeVisible();
  });

  test('the project sidebar "Intake" link navigates to it', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    const id = await firstProjectId();
    await navigateToScreen(tauriPage, `/project/${id}/overview`, `a[href="/project/${id}/intake"]`);
    await tauriPage.locator(`a[href="/project/${id}/intake"]`).first().click();
    await new Promise((r) => setTimeout(r, 800));
    expect(await pathname(tauriPage)).toBe(`/project/${id}/intake`);
  });

  test('freeform → recommend → confirm records a run', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    const id = await firstProjectId();
    await navigateToScreen(tauriPage, `/project/${id}/intake`, '[data-testid="intake-input"]');

    // The tauri-playwright `fill` helper drives HTMLInputElement.value, which
    // throws on a <textarea>; set the value via the native textarea setter and
    // dispatch `input` so Svelte's bind:value picks it up.
    await tauriPage.evaluate(`
      (function() {
        var el = document.querySelector('[data-testid="intake-input"]');
        var setter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set;
        setter.call(el, 'fix the null deref when the token refreshes');
        el.dispatchEvent(new Event('input', { bubbles: true }));
      })()
    `);
    await new Promise((r) => setTimeout(r, 300)); // let the disabled-state derive settle
    await tauriPage.locator('[data-testid="intake-recommend"]').click();

    // Classify (gateway, heuristic fallback) + recommend can take a few seconds.
    await expect(tauriPage.locator('[data-testid="intake-card"]')).toBeVisible({ timeout: 30_000 });
    const title = ((await tauriPage.evaluate(
      `(document.querySelector('[data-testid="intake-playbook-title"]') || {}).textContent || ''`,
    )) as string).trim();
    expect(title.length, `playbook title should not be empty`).toBeGreaterThan(0);
    const axes = ((await tauriPage.evaluate(
      `(document.querySelector('[data-testid="intake-axes"]') || {}).textContent || ''`,
    )) as string).trim();
    expect(axes.length, `axis chips should render`).toBeGreaterThan(0);

    await tauriPage.locator('[data-testid="intake-confirm"]').click();
    await expect(tauriPage.locator('[data-testid="intake-recorded"]')).toBeVisible({ timeout: 15_000 });
  });

  test('daemon recommend is project-scoped (accepts project_id, refuses a project-less run)', async () => {
    const id = await firstProjectId();
    const guide = await daemonGet<Record<string, unknown>>('/api/playbook/guide');
    expect(guide).toHaveProperty('playbooks');
    expect(guide).toHaveProperty('axes');
    expect(guide).toHaveProperty('frame');

    const rec = await daemonPost<Record<string, unknown>>('/api/playbook/recommend', {
      chunk: 'add a small feature flag to gate the new panel',
      preview: true,
      project_id: id,
    });
    expect(rec).toHaveProperty('playbook');
    expect(rec).toHaveProperty('lifecycle');
    expect(rec).toHaveProperty('intent');
    expect(rec).toHaveProperty('risk');
    expect(rec).toHaveProperty('auto_select');

    // A run always happens in a project — no project → an explicit error, never a
    // fabricated global recommendation.
    const err = await daemonPost<Record<string, unknown>>('/api/playbook/recommend', {
      chunk: 'add a small feature flag to gate the new panel',
      preview: true,
    });
    expect(err).toHaveProperty('error');
  });
});
