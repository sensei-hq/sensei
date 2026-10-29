/**
 * Inspect the REAL Schema diagram (#221) for this repository's workspace. Done
 * gate 1: `ErDiagram` and `DependencyDiagram` MOUNT against the live payload
 * (every table and key in its payload) without a runtime throw — the e2e DB has no
 * schema to draw, so only this harness can show it. Captures
 * /tmp/sensei-shots/schema-*.png. Run: `bun run test:inspect`.
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateTo, navigateToScreen, installErrorTrap, readErrors, DAEMON_URL, shot } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const settle = (ms = 3000) => new Promise<void>((r) => setTimeout(r, ms));

async function idForName(name: string): Promise<string | null> {
  const res = await fetch(`${DAEMON_URL}/api/workspaces`);
  const body = await res.json();
  const list: Array<{ id: string; name: string }> = Array.isArray(body) ? body : (body.workspaces ?? []);
  return list.find((p) => p.name === name)?.id ?? null;
}

const text = (tauriPage: TauriPageLike, sel: string) =>
  tauriPage.evaluate(
    `document.querySelector('${sel}')?.textContent?.replace(/\\s+/g,' ').trim() ?? null`,
  ) as Promise<string | null>;

/** A screenshot when the app has an on-screen window; this harness can also run
 *  where it has none, and the DOM counts below are the evidence either way. */
async function snap(tauriPage: TauriPageLike, name: string): Promise<void> {
  try {
    await shot(tauriPage, `${DIR}/${name}.png`);
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
    test.skip(true, 'no workspace named "sensei" on the real daemon');
    return;
  }
  await navigateTo(tauriPage, '/logs');
  await installErrorTrap(tauriPage);

  await navigateToScreen(tauriPage, `/workspace/${id}/diagrams/schema`, '[data-testid="schema-screen"]');
  await tauriPage.waitForSelector('[data-testid="schema-diagram"], [data-testid="schema-error"], [data-testid="schema-none"]', 60_000);
  await settle();
  // Live data grows: the screen must say what the daemon's payload holds, not a past count.
  type End = { s: string };
  /** Counts as the screen writes them (lib/format: thousands grouped). */
  const n = (v: number) => v.toLocaleString('en-US');
  const payload = (await (await fetch(`${DAEMON_URL}/api/workspaces/${id}/diagrams/schema`)).json()) as {
    tables: { schema: string }[];
    refs: { from: End; to: End }[];
    entities: { schema: string }[];
    deps: { from: End; to: End; unresolved?: boolean }[];
  };
  // One schema's share, derived here from the payload rather than taken from the screen:
  // links with both ends inside are drawn, links with one end outside are counted as cut.
  const share = (S: string, links: { from: End; to: End }[]) => ({
    inside: links.filter((l) => l.from.s === S && l.to.s === S).length,
    cut: links.filter((l) => (l.from.s === S) !== (l.to.s === S)).length,
  });
  const dojoTables = payload.tables.filter((t) => t.schema === 'dojo').length;
  const dojoObjects = dojoTables + payload.entities.filter((e) => e.schema === 'dojo').length;
  const dojoKeys = share('dojo', payload.refs);
  // A dependency on a built-in or undescribed object has no far end to place: it is drawn
  // (dimmed) whenever its source is, and never counted as cut.
  const resolved = payload.deps.filter((d) => !d.unresolved);
  const dimmedIn = (S: string | null) => payload.deps.filter((d) => d.unresolved && (S === null || d.from.s === S)).length;
  const dojoDeps = { inside: share('dojo', resolved).inside + dimmedIn('dojo'), cut: share('dojo', resolved).cut };
  const coverage = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (all): ${coverage}`);
  expect(coverage).toContain(`${n(payload.tables.length)} tables`);
  expect(coverage).toContain(`${n(payload.refs.length)} keys`);
  const all = await drawn(tauriPage);
  console.log(`[inspect] drawn (all): ${JSON.stringify(all)}`);
  expect(all.nodes, 'one card per table').toBe(payload.tables.length);
  expect(all.edges).toBeGreaterThan(0);
  await snap(tauriPage, 'schema-er-all');

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
  await snap(tauriPage, 'schema-er-selected');

  // One schema: only its tables, and the keys leading out of it counted.
  await pick(tauriPage, 'Schema', 'dojo');
  await settle();
  const filtered = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (dojo): ${filtered}`);
  // Done gate 2, against counts derived independently from the live payload (above).
  expect(filtered).toContain(`${n(dojoTables)} tables · ${n(dojoKeys.inside)} keys`);
  expect(filtered).toContain(`${n(dojoKeys.cut)} keys lead out of the shown schema, not drawn`);
  const dojo = await drawn(tauriPage);
  console.log(`[inspect] drawn (dojo): ${JSON.stringify(dojo)}`);
  expect(dojo.nodes, 'only the chosen schema').toBe(dojoTables);
  await snap(tauriPage, 'schema-er-dojo');

  await pick(tauriPage, 'Show', 'Dependencies');
  await settle();
  const deps = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (deps, dojo): ${deps}`);
  expect(deps).toContain(`${n(dojoObjects)} objects · ${n(dojoDeps.inside)} dependencies`);
  expect(deps).toContain(`${n(dojoDeps.cut)} dependencies cross out of the shown schema, not drawn`);
  expect(deps).toContain(`${n(dimmedIn('dojo'))} dependencies on a built-in or undescribed object, drawn dimmed`);
  await snap(tauriPage, 'schema-deps-dojo');

  await pick(tauriPage, 'Schema', 'All');
  await settle();
  const allDeps = await text(tauriPage, '[data-testid="schema-coverage"]');
  console.log(`[inspect] schema coverage (deps, all): ${allDeps}`);
  expect(allDeps).toContain(`${n(payload.tables.length + payload.entities.length)} objects · ${n(payload.deps.length)} dependencies`);
  expect(allDeps).toContain(`${n(dimmedIn(null))} dependencies on a built-in or undescribed object, drawn dimmed`);
  await snap(tauriPage, 'schema-deps-all');

  const errs = await readErrors(tauriPage);
  console.log(`[inspect] runtime errors: ${JSON.stringify(errs)}`);
  expect(errs.error, 'uncaught errors').toEqual([]);
  expect(errs.rejection, 'unhandled rejections').toEqual([]);
});
