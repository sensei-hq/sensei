/**
 * #293 against the REAL app: a screen on the shared `Screen` layout reaches the
 * bottom of its window's <main> (a layout may keep tabs above it), <main> itself never scrolls, and the header stays in place
 * while the body scrolls. The body is made taller than the window first, so the
 * probe holds even for a screen with little data.
 * Run: `bun run test:inspect -- screen-shell`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

/** Project-scoped screens on `Screen`; grows as screens move (#293). */
const PROJECT_SCREENS = [
  'traceability', 'instruments', 'libraries', 'memories',
  'diagrams/cycles', 'diagrams/layers', 'diagrams/neighbourhood', 'diagrams/schema', 'diagrams/structure', 'diagrams/world',
  'overview', 'about', 'patterns', 'impact', 'intake', 'metrics', 'sessions', 'atlas',
];

type Probe = { fills: boolean; mainScrolls: boolean; headerMoved: number; bodyScrolled: number; fill: boolean } | null;

/** What every screen must show: a `fill` screen's panes scroll themselves, so its
 *  body is not scrolled; any other body scrolls under a header that stays put. */
const expected = (fill: boolean) => ({ fills: true, mainScrolls: false, headerMoved: 0, bodyScrolled: fill ? 0 : 1500, fill });

/** A body not yet probed: the screen just navigated to, not the one before it. */
const FRESH_BODY = '[data-component="screen-body"]:not([data-probed])';

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
  test.setTimeout(180_000);
  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');

  const results: Record<string, Probe> = {};
  const metrics: { metrics: { metric: string }[] } = await (
    await fetch(`${DAEMON_URL}/api/projects/${sensei!.id}/metrics`)
  ).json();
  const screens = [...PROJECT_SCREENS, ...metrics.metrics.slice(0, 1).map((m) => `metrics/${m.metric}`)];
  for (const screen of screens) {
    await navigateToScreen(tauriPage, `/project/${sensei!.id}/${screen}`, FRESH_BODY, 30_000);
    results[screen] = (await tauriPage.evaluate(PROBE)) as Probe;
  }
  console.log(`\n[inspect] screen shell ${JSON.stringify(results)}`);
  expect(screens.length).toBeGreaterThan(PROJECT_SCREENS.length);
  for (const screen of screens) {
    expect(results[screen], screen).toEqual(expected(results[screen]?.fill ?? false));
  }
  // The two list-and-detail screens are the fill ones.
  expect(screens.filter((s) => results[s]?.fill)).toEqual(['atlas', screens.at(-1)]);
});
