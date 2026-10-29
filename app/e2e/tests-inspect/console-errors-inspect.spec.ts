/**
 * Console errors, screen by screen, against the REAL daemon (user, 2026-10-27:
 * keep track of console errors). Visits every main screen with the console
 * watched and reports what each one logged; any error fails it.
 * Run: `bun run test:inspect -- console-errors`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo, takeConsoleErrors, watchConsole } from '../helpers';

const OBSERVATORY = ['/', '/workspaces', '/instruments', '/instruments?tab=replay', '/instruments?tab=insights',
  '/libraries', '/metrics', '/sessions', '/settings/general'];
const WORKSPACE = ['overview', 'metrics', 'sessions', 'memories', 'traceability', 
  'diagrams/structure', 'diagrams/wheel', 'diagrams/zones', 'diagrams/matrix', 'diagrams/cycles',
  'diagrams/coupling', 'diagrams/complexity', 'diagrams/schema',
  'libraries', 'instruments', 'patterns', 'impact', 'tickets', 'integrations', 'about', 'intake'];

test('no screen logs a console error', async ({ tauriPage }) => {
  const workspaces = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const rows = Array.isArray(workspaces) ? workspaces : workspaces.workspaces;
  const ws = rows.find((w: { name: string }) => w.name === 'sensei') ?? rows[0];

  await watchConsole(tauriPage);
  await takeConsoleErrors(tauriPage); // what the boot logged is its own row
  const report: Record<string, string[]> = {};
  const visit = async (route: string) => {
    await navigateTo(tauriPage, route);
    await new Promise((r) => setTimeout(r, 2500));
    const errs = await takeConsoleErrors(tauriPage);
    if (errs.length) report[route] = [...new Set(errs.map((e) => `${e.kind}: ${e.text}`))];
  };
  for (const r of OBSERVATORY) await visit(r);
  for (const s of WORKSPACE) await visit(`/workspace/${ws.id}/${s}`);

  console.log(`\n[inspect] console errors ${JSON.stringify(report, null, 1)}`);
  expect(report).toEqual({});
});
