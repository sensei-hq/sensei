/**
 * #288 against the REAL daemon: the reads the setup wizard owns land — the
 * Settings Instruments section shows one card per daemon MCP and the Libraries
 * section the daemon's libraries for the scanned repositories — with no error.
 * Read from the DOM: the sign-in overlay (#283) is modal while it is up.
 * Run: `bun run test:inspect -- wizard-reads`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

const json = async (path: string) => (await fetch(`${DAEMON_URL}${path}`)).json();
const errors = `document.querySelectorAll('[data-testid="libraries-error"], [data-testid="instruments-error"], [data-screen-state="error"]').length`;

test('wizard-owned reads land', async ({ tauriPage }) => {
  const { mcps } = await json('/api/instruments');
  const projects: { folders?: { name: string }[] }[] = await json('/api/projects');
  const scanned = new Set(projects.flatMap((p) => (p.folders ?? []).map((f) => f.name)));
  const { libs } = await json('/api/libs?');
  const expectedLibs = libs.filter((l: { repos?: string[] }) => (l.repos ?? []).some((r) => scanned.has(r))).length;

  await navigateTo(tauriPage, '/settings/instruments');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-testid^="mcp-card-"]').length`), { timeout: 30_000 })
    .toBe(mcps.length);
  const inst = (await tauriPage.evaluate(errors)) as number;

  await navigateTo(tauriPage, '/settings/libraries');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-testid^="library-row-"]').length`), { timeout: 30_000 })
    .toBe(expectedLibs);
  const lib = (await tauriPage.evaluate(errors)) as number;

  console.log(`\n[inspect] wizard reads: mcps ${mcps.length}, libraries ${expectedLibs} of ${libs.length}; errors ${[inst, lib]}`);
  expect([inst, lib]).toEqual([0, 0]);
});
