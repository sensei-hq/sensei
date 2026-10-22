/**
 * #293 against the REAL app: a screen on the shared `Screen` layout reaches the
 * bottom of its window's <main> (a layout may keep tabs above it), <main> itself never scrolls, and the header stays in place
 * while the body scrolls. The body is made taller than the window first, so the
 * probe holds even for a screen with little data.
 * Run: `bun run test:inspect -- screen-shell`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL } from '../helpers';

/** Workspace-scoped screens on `Screen`; grows as screens move (#293). */
const WORKSPACE_SCREENS = [
  'traceability', 'instruments', 'libraries', 'memories',
  'diagrams/cycles', 'diagrams/layers', 'diagrams/neighbourhood', 'diagrams/schema', 'diagrams/structure', 'diagrams/world',
  'overview', 'about', 'patterns', 'impact', 'intake', 'metrics', 'sessions', 'atlas',
];

/** Observatory screens on `Screen`, by path; grows as screens move (#293). */
const OBSERVATORY_SCREENS = [
  ...['assistants', 'dojo', 'extensions', 'general', 'inference', 'instruments', 'libraries', 'metrics', 'workspaces', 'providers', 'roots']
    .map((s) => `/settings/${s}`),
  '/dojo/connections', '/dojo/sharing', '/help', '/knowledge-sources', '/libraries', '/traceability', '/upgrades',
  '/', '/impact', '/learnings', '/share-review',
  '/activity-logs', '/consolidation', '/insights', '/instruments', '/workspaces', '/sessions',
];

type Probe = { fills: boolean; mainScrolls: boolean; headerMoved: number; bodyScrolled: number; fill: boolean } | null;

/** What every screen must show: a `fill` screen's panes scroll themselves, so its
 *  body is not scrolled; any other body scrolls under a header that stays put. */
const expected = (fill: boolean) => ({ fills: true, mainScrolls: false, headerMoved: 0, bodyScrolled: fill ? 0 : 1500, fill });

/** A body not yet probed: the screen just navigated to, not the one before it. */
const FRESH_BODY = '[data-component="screen-body"]:not([data-probed])';

type Page = { evaluate: (script: string) => Promise<unknown> };

/** Navigate and wait for the new screen's body to be in the DOM. The identities
 *  dialog (#283) is modal and holds navigation while a credential needs
 *  approval, so it is closed first — it can open at any point, once the
 *  personas load. */
async function openScreen(page: Page, route: string) {
  await page.evaluate(
    `document.querySelector('[data-component="sign-in-overlay"] footer button')?.click()`,
  );
  // Navigated ONCE and not awaited: re-issuing restarts a slow loader, and
  // awaiting the navigation inside evaluate trips the harness's 30s call limit
  // on a screen whose load is slower than that (workspace Instruments, #298).
  await page.evaluate(`(() => {
    const a = document.createElement('a');
    a.href = ${JSON.stringify(route)};
    document.body.appendChild(a);
    a.click();
    a.remove();
  })()`);
  for (const until = Date.now() + 120_000; Date.now() < until; ) {
    // A page busy mounting (atlas lays its graph out on the main thread) cannot
    // answer; that is "not yet", not a failure.
    const mounted = await page.evaluate(`!!document.querySelector('${FRESH_BODY}')`).catch(() => false);
    if (mounted) return;
    await new Promise((r) => setTimeout(r, 250));
  }
  const where = await page.evaluate(
    `JSON.stringify({ at: location.pathname, bodies: document.querySelectorAll('[data-component="screen-body"]').length, probed: document.querySelectorAll('[data-probed]').length, dialog: !!document.querySelector('[role="dialog"], dialog[open]') })`,
  );
  throw new Error(`${route} never mounted a new screen body within 120s: ${where}`);
}

const PROBE = `(() => {
  const body = document.querySelector('[data-component="screen-body"]:not([data-probed])');
  const screen = body?.closest('[data-component="screen"]');
  const header = screen?.querySelector(':scope > [data-component="screen-header"]');
  const main = screen?.closest('main');
  if (!main || !screen || !header || !body) return null;
  const fill = body.dataset.fill === 'true';
  const spacer = document.createElement('div');
  spacer.style.height = '4000px';
  if (!fill) body.appendChild(spacer);
  const before = header.getBoundingClientRect().top;
  if (!fill) body.scrollTop = 1500;
  const out = {
    fills: Math.abs(screen.getBoundingClientRect().bottom - main.getBoundingClientRect().bottom) <= 1,
    mainScrolls: main.scrollHeight > main.clientHeight + 1,
    headerMoved: header.getBoundingClientRect().top - before,
    bodyScrolled: body.scrollTop,
    fill,
  };
  body.scrollTop = 0;
  spacer.remove();
  // Marked, so the next navigation waits for ITS screen rather than this one.
  body.setAttribute('data-probed', '');
  return out;
})()`;

test('screens on the shared layout keep the header fixed over one scroll body', async ({ tauriPage }) => {
  test.setTimeout(600_000);
  const workspaces: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const sensei = workspaces.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei workspace on this daemon');

  const results: Record<string, Probe> = {};
  const metrics: { metrics: { metric: string }[] } = await (
    await fetch(`${DAEMON_URL}/api/workspaces/${sensei!.id}/metrics`)
  ).json();
  const screens = [...WORKSPACE_SCREENS, ...metrics.metrics.slice(0, 1).map((m) => `metrics/${m.metric}`)];
  for (const screen of screens) {
    await openScreen(tauriPage, `/workspace/${sensei!.id}/${screen}`);
    results[screen] = (await tauriPage.evaluate(PROBE)) as Probe;
  }
  for (const path of OBSERVATORY_SCREENS) {
    await openScreen(tauriPage, path);
    results[path] = (await tauriPage.evaluate(PROBE)) as Probe;
  }
  console.log(`\n[inspect] screen shell ${JSON.stringify(results)}`);
  expect(screens.length).toBeGreaterThan(WORKSPACE_SCREENS.length);
  for (const screen of [...screens, ...OBSERVATORY_SCREENS]) {
    expect(results[screen], screen).toEqual(expected(results[screen]?.fill ?? false));
  }
  // The two list-and-detail screens are the fill ones.
  expect(screens.filter((s) => results[s]?.fill)).toEqual(['atlas', screens.at(-1)]);
});
