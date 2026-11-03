/**
 * Inspect the REAL Database · ER (#221) for this repository's workspace — dbd's
 * viewer from @rokkit/graph/schema (user, 2026-11-02): the design's Overview,
 * Diagram and Entities, an entity's page on selection, the sidebar on the right,
 * against the live payload (tables, keys, objects, enums). The e2e DB has no schema
 * to draw, so only this harness can show it. Captures /tmp/sensei-shots/schema-*.png.
 * Run: `bun run test:inspect`.
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

/** Open the sidebar entry named `name` (a rokkit Tree row) and let its page render. */
async function open(tauriPage: TauriPageLike, name: string): Promise<void> {
  await tauriPage.evaluate(`(() => {
    const row = [...document.querySelectorAll('[data-testid="schema-sidebar"] [data-tree-item-content]')]
      .find((n) => n.textContent.trim() === ${JSON.stringify(name)});
    row?.click();
  })()`);
  await settle(1500);
}

test('schema — sensei, dbd’s viewer over live data', async ({ tauriPage }) => {
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
  await tauriPage.waitForSelector('[data-testid="schema-layout"], [data-testid="schema-error"], [data-testid="schema-none"]', 60_000);
  await settle();
  const payload = (await (await fetch(`${DAEMON_URL}/api/workspaces/${id}/diagrams/schema`)).json()) as {
    version: number;
    tables: { schema: string; name: string }[];
    entities: { schema: string; name: string }[];
    enums?: { schema: string; name: string; values: string[] }[];
  };
  const n = (v: number) => v.toLocaleString('en-US');

  // The counts are the header's stat cards (user, 2026-11-03) — not the Overview's —
  // and the sidebar has no design-name row: the page is already the workspace's.
  const card = (label: string) => text(tauriPage, `[data-testid="diagram-stat-${label}"] [data-stat-card-value]`);
  await tauriPage.evaluate(`(() => {
    const t = document.querySelector('[data-testid="diagram-stats-toggle"]');
    if (t?.getAttribute('aria-pressed') === 'false') t.click();
  })()`);
  await settle();
  const cards = await text(tauriPage, '[data-testid="diagram-stats"]');
  console.log(`[inspect] schema stats: ${cards?.replace(/\s+/g, ' ').slice(0, 300)}`);
  expect(await card('Tables')).toBe(n(payload.tables.length));
  if (payload.enums?.length) expect(await card('Enums')).toBe(n(payload.enums.length));
  expect(await tauriPage.evaluate(`!!document.querySelector('[data-graph-section="counts"]')`)).toBe(false);
  expect(await tauriPage.evaluate(`!!document.querySelector('[data-testid="schema-sidebar"] [data-graph-design-home]')`)).toBe(false);
  const sideIsRight = (await tauriPage.evaluate(
    `document.querySelector('[data-testid="schema-layout"]')?.lastElementChild?.getAttribute('data-testid') === 'schema-sidebar'`,
  )) as boolean;
  expect(sideIsRight, 'the sidebar sits on the right').toBe(true);
  await snap(tauriPage, 'schema-overview');

  // The Diagram tab draws every table.
  await tauriPage.evaluate(`(() => {
    const tab = [...document.querySelectorAll('[data-graph-design-header] [role="tab"], [data-graph-design-header] button')]
      .find((t) => t.textContent.trim() === 'Diagram');
    tab?.click();
  })()`);
  await settle(4000);
  const nodes = (await tauriPage.evaluate(
    `document.querySelectorAll('[data-testid="schema-diagram"] [data-graph-node]').length`,
  )) as number;
  console.log(`[inspect] schema diagram: ${nodes} nodes for ${payload.tables.length} tables`);
  expect(nodes).toBeGreaterThanOrEqual(payload.tables.length);
  await snap(tauriPage, 'schema-diagram');

  // A table's page, and an enum's with its values.
  const aTable = payload.tables[0];
  await open(tauriPage, aTable.name);
  expect(await tauriPage.evaluate(`document.querySelector('[data-testid="schema-page"]')?.getAttribute('data-page')`)).toBe('table');
  await snap(tauriPage, 'schema-table');
  const anEnum = payload.enums?.[0];
  if (anEnum) {
    await open(tauriPage, anEnum.name);
    expect(await tauriPage.evaluate(`document.querySelector('[data-testid="schema-page"]')?.getAttribute('data-page')`)).toBe('enum');
    const page = await text(tauriPage, '[data-testid="schema-page"]');
    expect(page).toContain(anEnum.values[0]);
    await snap(tauriPage, 'schema-enum');
  } else {
    console.log('[inspect] no enums in the payload — the daemon predates v3 enums, or the design has none');
  }

  const errs = await readErrors(tauriPage);
  console.log(`[inspect] runtime errors: ${JSON.stringify(errs)}`);
  expect(errs.error, 'uncaught errors').toEqual([]);
  expect(errs.rejection, 'unhandled rejections').toEqual([]);
});
