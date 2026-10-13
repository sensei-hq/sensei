/**
 * Inspect the REAL Schema diagram (#221) for this repository's project. Done
 * gate 1: `ErDiagram` and `DependencyDiagram` MOUNT against the live payload
 * (169 tables, 184 keys, 784 deps) without a runtime throw — the e2e DB has no
 * schema to draw, so only this harness can show it. Captures
 * /tmp/sensei-shots/schema-*.png. Run: `bun run test:inspect`.
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateTo, navigateToScreen, installErrorTrap, readErrors, DAEMON_URL } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const settle = (ms = 3000) => new Promise<void>((r) => setTimeout(r, ms));

async function idForName(name: string): Promise<string | null> {
  const res = await fetch(`${DAEMON_URL}/api/projects`);
  const body = await res.json();
  const list: Array<{ id: string; name: string }> = Array.isArray(body) ? body : (body.projects ?? []);
  return list.find((p) => p.name === name)?.id ?? null;
}

const text = (tauriPage: TauriPageLike, sel: string) =>
  tauriPage.evaluate(
    `document.querySelector('${sel}')?.textContent?.replace(/\\s+/g,' ').trim() ?? null`,
  ) as Promise<string | null>;

/** A screenshot when the app has an on-screen window; this harness can also run
 *  where it has none, and the DOM counts below are the evidence either way. */
async function shot(tauriPage: TauriPageLike, name: string): Promise<void> {
  try {
    await tauriPage.screenshot({ path: `${DIR}/${name}.png` });
  } catch (e) {
    console.log(`[inspect] no screenshot ${name}: ${(e as Error).message}`);
  }
}

/** Cards and lines the canvas actually drew. */
async function drawn(tauriPage: TauriPageLike): Promise<{ nodes: number; edges: number }> {
  return (await tauriPage.evaluate(`({
    nodes: document.querySelectorAll('[data-testid="schema-diagram"] [data-graph-node]').length,
    edges: document.querySelectorAll('[data-testid="schema-diagram"] [data-graph-edge]').length,
  })`)) as { nodes: number; edges: number };
}

/** Click the toolbar button whose label starts with `label`. The tauri bridge's
 *  locator engine has no `:has-text`, so the button is found in the page. */
async function pick(tauriPage: TauriPageLike, toolbar: string, label: string): Promise<void> {
  const clicked = await tauriPage.evaluate(`
    (() => {
      const bar = document.querySelector('[data-toolbar][aria-label="${toolbar}"]');
      const btn = [...(bar?.querySelectorAll('button') ?? [])]
        .find((b) => b.textContent.trim().startsWith(${JSON.stringify(label)}));
      btn?.click();
      return !!btn;
    })()
  `);
  expect(clicked, `${toolbar} → ${label}`).toBe(true);
}

test('schema — sensei, both scopes over live data', async ({ tauriPage }) => {
  test.setTimeout(240_000);
  mkdirSync(DIR, { recursive: true });
  const id = await idForName('sensei');
  if (!id) {
    test.skip(true, 'no project named "sensei" on the real daemon');
    return;
  }
  await navigateTo(tauriPage, '/logs');
  await installErrorTrap(tauriPage);

  await navigateToScreen(tauriPage, `/project/${id}/diagrams/schema`, '[data-testid="schema-screen"]');
  await tauriPage.waitForSelector('[data-testid="schema-diagram"], [data-testid="schema-error"], [data-testid="schema-none"]', 60_000);
  await settle();
  const coverage = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (all): ${coverage}`);
  expect(coverage).toContain('169 tables');
  expect(coverage).toContain('184 keys');
  const all = await drawn(tauriPage);
  console.log(`[inspect] drawn (all): ${JSON.stringify(all)}`);
  expect(all.nodes, 'one card per table').toBe(169);
  expect(all.edges).toBeGreaterThan(0);
  await shot(tauriPage, 'schema-er-all');

  // Done gate 3: selecting a table on the canvas shows its columns, in order.
  const hit = await tauriPage.evaluate(`
    (() => {
      const card = [...document.querySelectorAll('[data-testid="schema-diagram"] [data-graph-node]')]
        .find((n) => n.querySelector('[data-graph-node-title]')?.textContent?.trim() === 'schema_refs');
      card?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      return !!card;
    })()
  `);
  expect(hit, 'the schema_refs card is on the canvas').toBe(true);
  await settle(1500);
  const entity = await text(tauriPage, '[data-testid="schema-entity"]');
  console.log(`[inspect] entity panel: ${entity?.slice(0, 200)}`);
  expect(entity).toContain('from_node_id');
  expect(entity?.indexOf('from_column')).toBeGreaterThan(entity?.indexOf('from_node_id') ?? -1);
  await shot(tauriPage, 'schema-er-selected');

  // One schema: only its tables, and the keys leading out of it counted.
  await pick(tauriPage, 'Schema', 'dojo');
  await settle();
  const filtered = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (dojo): ${filtered}`);
  // Done gate 2, against counts derived independently from the live payload
  // (2026-10-12): dojo has 39 tables, 63 keys inside it and 4 with one end out.
  expect(filtered).toContain('39 tables · 63 keys');
  expect(filtered).toContain('4 keys lead out of the shown schema, not drawn');
  const dojo = await drawn(tauriPage);
  console.log(`[inspect] drawn (dojo): ${JSON.stringify(dojo)}`);
  expect(dojo.nodes, 'only the chosen schema').toBe(39);
  await shot(tauriPage, 'schema-er-dojo');

  await pick(tauriPage, 'Show', 'Dependencies');
  await settle();
  const deps = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (deps, dojo): ${deps}`);
  expect(deps).toContain('51 objects · 159 dependencies');
  expect(deps).toContain('18 dependencies cross out of the shown schema, not drawn');
  expect(deps).toContain('80 dependencies on a built-in or undescribed object, drawn dimmed');
  await shot(tauriPage, 'schema-deps-dojo');

  await pick(tauriPage, 'Schema', 'All');
  await settle();
  const allDeps = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (deps, all): ${allDeps}`);
  expect(allDeps).toContain('262 objects · 784 dependencies');
  expect(allDeps).toContain('360 dependencies on a built-in or undescribed object, drawn dimmed');
  await shot(tauriPage, 'schema-deps-all');

  const errs = await readErrors(tauriPage);
  console.log(`[inspect] runtime errors: ${JSON.stringify(errs)}`);
  expect(errs.error, 'uncaught errors').toEqual([]);
  expect(errs.rejection, 'unhandled rejections').toEqual([]);
});
