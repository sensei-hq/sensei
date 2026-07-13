/**
 * Observatory · Logs (/activity-logs) — functional e2e.
 *
 * The unit tests + svelte-check already cover the pure derivations. This spec
 * covers what they can't: the loader hitting the REAL e2e daemon, the rows
 * rendering with level/severity styling, and a server-side filter refetching
 * (URL update + list change) — with NO runtime throw (uncaught error / rejection
 * / console.error) during mount or interaction.
 *
 * Data strategy: this screen's own ingest endpoint (POST /api/logs) lets us
 * inject deterministic rows, so we do. A unique per-run `running_on` token
 * isolates our rows from any other logs already in the throwaway sensei_e2e DB
 * (and from a prior run of this spec), so the assertions are exact.
 */

import { test, expect } from '../fixtures';
import {
  navigateTo, navigateToScreen, waitForDom, DAEMON_URL,
  installErrorTrap, readErrors, type ErrBuf,
} from '../helpers';

// Unique run token → our injected rows are the ONLY rows with this source, so
// navigating with `?source=<RUN>` yields exactly the three we control.
const RUN = `e2e-logs-${Date.now()}`;
const MODULE = 'e2e-logtest';

/** Mark setup complete in both daemon + the running app so the observatory
 *  shell is reachable (not rerouted to /setup). Same pattern as the learnings /
 *  knowledge-sources specs; localStorage set as belt-and-suspenders for a cold
 *  webview (this spec sorts first alphabetically). */
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

/** POST one structured log row into the e2e daemon. */
async function ingestLog(level: string, message: string): Promise<void> {
  const res = await fetch(`${DAEMON_URL}/api/logs`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      level,
      running_on: RUN,
      logged_at: new Date().toISOString(),
      message,
      context: { module: MODULE },
      data: { probe: message },
      error: level === 'error' ? { detail: 'synthetic e2e failure' } : null,
    }),
  });
  if (!res.ok) throw new Error(`ingest ${level} log failed: ${res.status}`);
}

/** Assert the trap caught no uncaught error / rejection / console.error. */
function expectNoRuntimeErrors(errs: ErrBuf, where: string): void {
  expect(errs.error, `${where}: uncaught errors`).toEqual([]);
  expect(errs.rejection, `${where}: unhandled rejections`).toEqual([]);
  expect(errs.console, `${where}: console.error output`).toEqual([]);
}

test.describe('Observatory · Logs (/activity-logs)', () => {
  test.beforeAll(async () => {
    // Inject three rows spanning the severity ramp. Newest-first ordering means
    // error → warn → info top-to-bottom, but we assert on set membership.
    await ingestLog('info', `${RUN} info line`);
    await ingestLog('warn', `${RUN} warn line`);
    await ingestLog('error', `${RUN} error line`);
  });

  test.beforeEach(async ({ tauriPage }) => {
    // Headroom for the retry-navigate budget: the cold health-bootstrap +
    // setup-reconcile for the FIRST gated screen can take ~50s on a loaded box.
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA to a setup-exempt route
    await installErrorTrap(tauriPage);
  });

  test('renders injected rows with per-level severity styling', async ({ tauriPage }) => {
    // Retry-navigate: the first gated nav of the suite bears the cold
    // health-bootstrap + setup reconciliation (see navigateToScreen).
    await navigateToScreen(tauriPage, `/activity-logs?source=${RUN}`, '[data-screen="activity-logs"]');
    // Screen mounted → loader completed against the real daemon; rows follow.
    await tauriPage.waitForSelector('[data-component="log-row"]', 15_000);

    const levels = (await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-component="log-row"]'))
        .map(el => el.getAttribute('data-log-level'))
    `)) as string[];
    // Exactly our three rows (unique source), covering error/warn/info.
    expect(levels.sort()).toEqual(['error', 'info', 'warn']);

    // Count note reflects the loaded set.
    const count = (await tauriPage.textContent('[data-log-count]')) ?? '';
    expect(count).toContain('showing 3 logs');

    // Severity styling: the error row's level chip carries the error tone class.
    const errorChipClass = await tauriPage.getAttribute(
      '[data-log-level="error"] [data-log-level-chip]',
      'class',
    );
    expect(errorChipClass ?? '').toContain('text-error');
    const errorChipText = await tauriPage.textContent(
      '[data-log-level="error"] [data-log-level-chip]',
    );
    expect((errorChipText ?? '').trim()).toBe('error');

    expectNoRuntimeErrors(await readErrors(tauriPage), 'render');
  });

  test('changing the level filter refetches (URL updates + list narrows)', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, `/activity-logs?source=${RUN}`, '[data-screen="activity-logs"]');
    await tauriPage.waitForSelector('[data-component="log-row"]', 15_000);

    const before = (await tauriPage.evaluate(
      `document.querySelectorAll('[data-component="log-row"]').length`,
    )) as number;
    expect(before).toBe(3);

    // Drive the level <select> exactly as the user would: set value + fire the
    // native change event the Svelte onchange handler binds to. This triggers
    // apply() → goto() → the loader refetch (server-side filter via the URL).
    await tauriPage.evaluate(`
      (function() {
        var sel = document.querySelector('[data-log-filter-level]');
        sel.value = 'error';
        sel.dispatchEvent(new Event('change', { bubbles: true }));
      })()
    `);

    // URL carries the new server filter…
    await waitForDom(tauriPage, `window.location.search.indexOf('level=error') !== -1`);
    // …and the list narrows to the single error row.
    await waitForDom(tauriPage, `document.querySelectorAll('[data-component="log-row"]').length === 1`);
    const afterLevels = (await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-component="log-row"]'))
        .map(el => el.getAttribute('data-log-level'))
    `)) as string[];
    expect(afterLevels).toEqual(['error']);

    expectNoRuntimeErrors(await readErrors(tauriPage), 'filter refetch');
  });
});
