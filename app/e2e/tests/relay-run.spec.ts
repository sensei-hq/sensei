/**
 * The relay run, driven from the app end to end (#426; design 2026-11-12 §3): the
 * person works the app, a fake crew driver plays the lanes over the lane HTTP API
 * (`e2e/lib/relay-crew.ts`), and every screen change after the first navigation
 * arrives from the stream — no reload, no retry. This is the gate for calling Relay
 * working.
 *
 * One test, one screen session — the app's test fixture resets the screen between
 * tests, and the journey's point is that nothing reloads. Each stage is a step:
 *   1. Queue  — two open items added, sent to the backlog, picked; Create run drafts it
 *               (no crew yet).
 *   2. Crew   — the driver registers a coordinator and two contributors: their cards
 *               appear live; Start starts the draft; the run card reads planning.
 */
import { execFileSync } from 'child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateToScreen, seedSetupComplete, DAEMON_URL } from '../helpers';
import { RelayCrew, type RelayLane } from '../lib/relay-crew';

const STREAM = 20_000;
const REPO_KEY = 'github.com/sensei-hq/e2e-run';
// Unique per run: the shared e2e broker keeps every earlier run's items.
const RUN_TAG = Date.now().toString(36);
const ITEMS = [`Run journey ${RUN_TAG}: the first file`, `Run journey ${RUN_TAG}: the second file`];

/** A person route on the daemon (no lane token). */
async function person<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${method} ${path}: HTTP ${res.status}: ${await res.text()}`);
  return (await res.json()) as T;
}

/** Earlier specs leave crews and runs open on the shared e2e broker: end them, so this
 *  journey starts with no crew (Create run drafts) and one crew after it registers. */
async function cleanSlate(): Promise<void> {
  const runs = await person<{ active: Array<{ id: string }> }>('GET', '/relay/runs');
  for (const run of runs.active) await person('POST', `/relay/runs/${run.id}/cancel`, {});
  const now = await person<{ crews: Array<{ id: string }> }>('GET', '/relay/now');
  for (const crew of now.crews) await person('POST', `/relay/crews/${crew.id}/close`, {});
  // Backlog and triage leftovers would be picked alongside ours; leave them, the
  // journey picks its own rows by title.
}

let repo = '';
let tauri: TauriPageLike;
const crew = new RelayCrew(DAEMON_URL);
const lanes: Record<'coord' | 'a' | 'b', RelayLane | null> = { coord: null, a: null, b: null };

// One selector per control, by its own accessible name: this driver does not scope a
// chained or filtered locator to its row — it acts on the page's first match.
const css = (v: string) => v.replace(/["\\]/g, '\\$&');
const addButton = (title: string) => tauri.locator(`[data-relay-section="triage"] [aria-label="Add to backlog: ${css(title)}"]`);
const pickBox = (title: string) => tauri.locator(`[data-relay-section="backlog"] [aria-label="Pick for a run: ${css(title)}"]`);
/** Set a field's value as typing would (an input event), for what `fill` cannot set — a textarea. */
const typeInto = (selector: string, value: string) =>
  tauri.evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) throw new Error('no element for ' + ${JSON.stringify(selector)});
    const proto = Object.getPrototypeOf(el);
    Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, ${JSON.stringify(value)});
    el.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
/** How many elements a selector matches, counted in the page (this driver has no count). */
const countOf = async (selector: string) =>
  Number(await tauri.evaluate(`document.querySelectorAll(${JSON.stringify(selector)}).length`));
/** The text of every element a selector matches, read in the page. */
const textsOf = (selector: string) =>
  tauri.evaluate(`JSON.stringify([...document.querySelectorAll(${JSON.stringify(selector)})].map((e) => e.textContent.replace(/\\s+/g, ' ').trim()))`) as Promise<string>;
const button = (label: string) => tauri.locator(`[data-screen-tools] [aria-label="${label}"]`);
/** A view button: its accessible name starts with the view (a count may follow). */
const view = (name: 'Now' | 'Queue' | 'Crew') => tauri.locator(`[data-screen-tools] [aria-label^="${name}"]`);

test.beforeAll(async () => {
  await cleanSlate();
  // The run's base: a throwaway repository with main and two lane branches, for the
  // broker's merge gate in a later stage.
  repo = mkdtempSync(join(tmpdir(), 'sensei-relay-run-'));
  const g = (...args: string[]) => execFileSync('git', ['-C', repo, ...args], { stdio: 'ignore' });
  g('init', '-q', '-b', 'main');
  g('config', 'user.email', 'e2e@sensei.test');
  g('config', 'user.name', 'e2e');
  writeFileSync(join(repo, 'README'), 'base\n');
  g('add', '.');
  g('commit', '-qm', 'init');
});

test.afterAll(async () => {
  for (const lane of Object.values(lanes)) await lane?.unregister().catch(() => {});
  if (repo) rmSync(repo, { recursive: true, force: true });
});

test('the run, driven from the app end to end, every change from the stream', async ({ tauriPage }) => {
  test.setTimeout(600_000);
  tauri = tauriPage;
  await test.step('1 · Queue: two items to the backlog, picked; Create run drafts it (no crew yet)', () => queueStage(tauriPage));
  await test.step('2 · Crew: a coordinator and two contributors register and appear live; Start starts the draft', () => crewStage());
});

async function queueStage(tauriPage: TauriPageLike): Promise<void> {
  await seedSetupComplete(tauriPage);
  await navigateToScreen(tauriPage, '/relay', '[data-screen="relay"] [data-screen-tools]');
  await view('Queue').click();
  await expect(tauriPage.locator('[data-relay-section="triage"]')).toBeVisible({ timeout: STREAM });

  for (const title of ITEMS) {
    await tauriPage.locator('[data-add-item-title]').fill(title);
    await tauriPage.locator('[data-add-item-submit]').click();
    await expect(addButton(title)).toBeVisible({ timeout: STREAM });
    await addButton(title).click();
    await expect(pickBox(title)).toBeVisible({ timeout: STREAM });
    await expect.poll(() => countOf(`[data-relay-section="triage"] [aria-label="Add to backlog: ${css(title)}"]`), { timeout: STREAM }).toBe(0);
  }
  for (const title of ITEMS) await pickBox(title).click();

  await button('Create run · 2').click();
  const form = tauriPage.locator('[data-create-sprint]');
  await expect(form).toBeVisible();
  const field = (name: string) => `[data-create-sprint] [data-sprint-field="${name}"]`;
  await typeInto(field('title'), 'Run journey');
  await typeInto(field('goal'), 'Prove the run end to end');
  await typeInto(field('base_checkout'), repo);
  await typeInto(field('base_branch'), 'main');
  await typeInto(field('verify_cmd'), 'test -f README');
  await tauriPage.locator('[data-create-sprint] [data-sprint-submit]').click();
  await expect(form).toBeHidden({ timeout: STREAM });

  // No crew: the run waits as a draft on the Crew view, Start waiting for the crew.
  await view('Crew').click();
  await expect(tauriPage.locator('[data-run-card] [data-run-title]')).toContainText('draft', { timeout: STREAM });
  await expect(tauriPage.locator('[data-run-card] [data-run-meta]')).toContainText('Run journey');
  await expect(tauriPage.locator('[data-run-card] [data-run-start]')).toBeDisabled();
  await expect(tauriPage.locator('[data-run-card] [data-run-note]')).toHaveText('Set up your crew to start it.');
}

async function crewStage(): Promise<void> {
  lanes.coord = await crew.register({ role: 'coordinator', family: 'e2e', goal: 'run journey crew', repo_key: REPO_KEY, suggested_name: 'atlas-e2e' });
  await expect.poll(() => countOf('[data-member]'), { timeout: STREAM }).toBe(1);
  lanes.a = await crew.register({ family: 'e2e', crew_id: lanes.coord.crewId, suggested_name: 'cobalt-e2e' });
  lanes.b = await crew.register({ family: 'e2e', crew_id: lanes.coord.crewId, suggested_name: 'coral-e2e' });
  // Every member's card, by its codename, as the stream brings each in.
  await expect
    .poll(async () => {
      const cards = await textsOf('[data-member]');
      return ['atlas-e2e', 'cobalt-e2e', 'coral-e2e'].filter((name) => cards.includes(name));
    }, { timeout: STREAM })
    .toEqual(['atlas-e2e', 'cobalt-e2e', 'coral-e2e']);
  expect(await countOf('[data-member]')).toBe(3);

  const start = tauri.locator('[data-run-card] [data-run-start]');
  await expect(start).toBeEnabled({ timeout: STREAM });
  await start.click();
  await expect(tauri.locator('[data-run-card] [data-run-title]')).toContainText('planning', { timeout: STREAM });
  expect(await countOf('[data-run-card] [data-run-start]')).toBe(0);
}
