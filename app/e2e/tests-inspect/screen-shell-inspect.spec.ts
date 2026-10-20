/**
 * #293 against the REAL app: a screen on the shared `Screen` layout fills its
 * window's <main>, <main> itself never scrolls, and the header stays in place
 * while the body scrolls. The body is made taller than the window first, so the
 * probe holds even for a screen with little data.
 * Run: `bun run test:inspect -- screen-shell`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';

/** Project-scoped screens on `Screen`; grows as screens move (#293). */
const PROJECT_SCREENS = ['traceability'];

type Probe = { fills: boolean; mainScrolls: boolean; headerMoved: number; bodyScrolled: number } | null;

const PROBE = `(() => {
  const main = document.querySelector('main');
  const screen = document.querySelector('[data-component="screen"]');
  const header = document.querySelector('[data-component="screen-header"]');
  const body = document.querySelector('[data-component="screen-body"]');
  if (!main || !screen || !header || !body) return null;
  const spacer = document.createElement('div');
  spacer.style.height = '4000px';
  body.appendChild(spacer);
  const before = header.getBoundingClientRect().top;
  body.scrollTop = 1500;
  const out = {
    fills: Math.abs(screen.getBoundingClientRect().height - main.clientHeight) <= 1,
    mainScrolls: main.scrollHeight > main.clientHeight + 1,
    headerMoved: header.getBoundingClientRect().top - before,
    bodyScrolled: body.scrollTop,
  };
  body.scrollTop = 0;
  spacer.remove();
  return out;
})()`;

test('screens on the shared layout keep the header fixed over one scroll body', async ({ tauriPage }) => {
  test.setTimeout(180_000);
  const projects: { id: string; name: string }[] = await (await fetch(`${DAEMON_URL}/api/projects`)).json();
  const sensei = projects.find((p) => p.name === 'sensei');
  test.skip(!sensei, 'no sensei project on this daemon');

  const results: Record<string, Probe> = {};
  for (const screen of PROJECT_SCREENS) {
    await navigateToScreen(tauriPage, `/project/${sensei!.id}/${screen}`, '[data-component="screen-body"]', 30_000);
    results[screen] = (await tauriPage.evaluate(PROBE)) as Probe;
  }
  console.log(`\n[inspect] screen shell ${JSON.stringify(results)}`);
  for (const screen of PROJECT_SCREENS) {
    expect(results[screen], screen).toEqual({ fills: true, mainScrolls: false, headerMoved: 0, bodyScrolled: 1500 });
  }
});
