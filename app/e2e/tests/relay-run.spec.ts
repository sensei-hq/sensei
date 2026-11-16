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
 *   3. Planner — a lane claims Task 0 and asks the person; Needs you shows it and the
 *               person answers; the plan report floats for review; another lane approves
 *               it; the plan unpacks and the band reads active.
 *   4. Work   — both lanes claim (the second early, gated on the first); a checkpoint's
 *               steps show on the lane row; the first report waits for a reviewer, is
 *               approved, and the broker's gate merges it (Completed today); then the
 *               second, the same way.
 *   5. Done   — the run completes: it leaves the band, and Past runs lists it.
 */
import { execFileSync } from 'child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateToScreen, seedSetupComplete, DAEMON_URL } from '../helpers';
import { RelayCrew, type RelayLane } from '../lib/relay-crew';
import { shot } from '../lib/shot';

const STREAM = 20_000;
const EVIDENCE = 'test-results/relay-run';
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
/** A picture of a state, for the report: best effort — the window capture can come
 *  back unavailable in this harness (#284), so its verdict is logged, never asserted. */
async function evidence(name: string): Promise<void> {
  mkdirSync(EVIDENCE, { recursive: true });
  await new Promise((r) => setTimeout(r, 1200));
  console.log(`[relay-run] ${name}: ${await shot(tauri, `${EVIDENCE}/${name}.png`)}`);
}
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
  // One branch per planned task, as each lane would push it.
  for (const t of ['t1', 't2']) {
    g('checkout', '-qb', `lane/${t}`, 'main');
    writeFileSync(join(repo, `${t}.txt`), `${t}\n`);
    g('add', '.');
    g('commit', '-qm', t);
  }
  g('checkout', '-q', 'main');
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
  await test.step('3 · Planner: Task 0 claimed, the person answers its question, the plan is reviewed and unpacks', () => plannerStage());
  await test.step('4 · Work: claims, steps, a report reviewed and merged by the gate — twice', () => workStage());
  await test.step('5 · Done: the run completes, leaves the band, and Past runs lists it', () => doneStage());
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
  await evidence('1-queue-draft');
}

async function crewStage(): Promise<void> {
  lanes.coord = await crew.register({ role: 'coordinator', family: 'e2e', goal: 'run journey crew', repo_key: REPO_KEY, suggested_name: 'atlas-e2e' });
  await expect.poll(() => countOf('[data-member]'), { timeout: STREAM }).toBe(1);
  lanes.a = await crew.register({ family: 'e2e', crew_id: lanes.coord.crewId, suggested_name: 'cobalt-e2e' });
  lanes.b = await crew.register({ family: 'e2e', crew_id: lanes.coord.crewId, suggested_name: 'coral-e2e' });
  // Every member's card, by its codename, as the stream brings each in.
  const roster = async () => {
    const cards: string[] = JSON.parse(await textsOf('[data-member]'));
    return { cards: cards.length, names: ['atlas-e2e', 'cobalt-e2e', 'coral-e2e'].filter((n) => cards.some((c) => c.includes(n))) };
  };
  const full = { cards: 3, names: ['atlas-e2e', 'cobalt-e2e', 'coral-e2e'] };
  await expect.poll(roster, { timeout: STREAM }).toEqual(full);
  // Once all three are in, the roster holds: a read that was out while a lane
  // registered lands without dropping it (#450 — it used to blink back to two).
  for (let i = 0; i < 10; i += 1) {
    expect(await roster()).toEqual(full);
    await new Promise((r) => setTimeout(r, 200));
  }
  // The Crew button's badge counts them too (its name carries the count).
  await expect.poll(() => attrOf('[data-screen-tools] [aria-label^="Crew"]', 'aria-label'), { timeout: STREAM }).toBe('Crew · 3');


  const start = tauri.locator('[data-run-card] [data-run-start]');
  await expect(start).toBeEnabled({ timeout: STREAM });
  await evidence('2-crew-draft-ready');
  await start.click();
  await expect(tauri.locator('[data-run-card] [data-run-title]')).toContainText('planning', { timeout: STREAM });
  expect(await countOf('[data-run-card] [data-run-start]')).toBe(0);
}

/** An attribute of the first element a selector matches, read in the page. */
const attrOf = (selector: string, name: string) =>
  tauri.evaluate(`document.querySelector(${JSON.stringify(selector)})?.getAttribute(${JSON.stringify(name)}) ?? null`) as Promise<string | null>;

const PLAN = {
  tasks: [
    { key: 't1', title: 'First file', phase: 'impl', owned_paths: ['t1.txt'], acceptance: ['t1.txt exists'], deps: [] },
    { key: 't2', title: 'Second file', phase: 'impl', owned_paths: ['t2.txt'], acceptance: ['t2.txt exists'], deps: ['t1'] },
  ],
};
const ids: { run: string; task0: string } = { run: '', task0: '' };

async function plannerStage(): Promise<void> {
  const a = lanes.a!;
  const b = lanes.b!;
  const runs = await person<{ active: Array<{ id: string; title: string; state: string }> }>('GET', '/relay/runs');
  ids.run = runs.active.find((r) => r.title === 'Run journey')!.id;

  await view('Now').click();
  const band = `[data-sprint-band][data-run-id="${ids.run}"]`;
  await expect.poll(() => attrOf(band, 'data-run-state'), { timeout: STREAM }).toBe('planning');

  // A lane claims Task 0: the run's group shows the plan in progress, by that lane (#422).
  const claim = await a.claimNext();
  expect(claim.claim_type).toBe('task');
  ids.task0 = claim.task!.id;
  const task0Row = `${runGroup()} [data-run-stage="active"] [data-run-task="${ids.task0}"]`;
  await expect.poll(async () => await textsOf(task0Row), { timeout: STREAM }).toContain('Plan: Run journey');
  expect(await textsOf(`${task0Row} [data-run-task-who]`)).toContain('cobalt-e2e');

  // It asks the person; Needs you shows the question, and the person answers in the app.
  const decisionId = await a.ask({
    task_id: ids.task0,
    question: 'One file per task, or both in one?',
    options: [
      { id: 'split', label: 'One per task' },
      { id: 'one', label: 'Both in one' },
    ],
    recommended: 'split',
  });
  const needs = '[data-relay-section="needs-you"]';
  await expect.poll(async () => (await textsOf(`${needs} [data-decision-question]`)).includes('One file per task, or both in one?'), { timeout: STREAM }).toBe(true);
  const card = `${needs} [data-decision="${decisionId}"]`;
  await tauri.locator(`${card} [data-option="split"]`).click();
  await tauri.locator(`${card} [data-decision-answer]`).click();
  await expect.poll(() => countOf(card), { timeout: STREAM }).toBe(0);

  // The plan report floats for review; another lane claims it and approves.
  await a.reportPlan(ids.task0, PLAN, 'Planned two files, one per task.');
  const floating = `[data-floating-review][data-task-id="${ids.task0}"]`;
  await expect.poll(() => countOf(floating), { timeout: STREAM }).toBe(1);
  const review = await b.claimNext();
  expect([review.claim_type, review.task?.id]).toEqual(['review', ids.task0]);
  const approved = await b.review(ids.task0, 'approved', 'Sound plan.');
  expect(approved.state).toBe('done');
  await expect.poll(() => countOf(floating), { timeout: STREAM }).toBe(0);

  // The plan unpacks: the band reads active.
  await expect.poll(() => attrOf(band, 'data-run-state'), { timeout: STREAM }).toBe('active');
  await evidence('3-planner-active');
}

/** Press By run or By agent (a filtered tauri locator acts on its first match, so by text here). */
const pickLaneView = (label: string) =>
  tauri.evaluate(`[...document.querySelectorAll('[data-lane-view] button')].find((b) => b.textContent.trim() === ${JSON.stringify(label)})?.click()`);

/** The run's group on Now, by run (#422). */
const runGroup = () => `[data-run-group="${ids.run}"]`;

/** The broker's merge gate runs on the sweeper's pass. */
const GATE = 200_000;
const work: { t1: string; t2: string } = { t1: '', t2: '' };

/** Wait for the gate to merge a task, then for the screen to show it under Completed today. */
async function mergedAndCompleted(taskId: string, title: string): Promise<void> {
  await expect
    .poll(async () => (await person<{ task: { state: string } }>('GET', `/relay/tasks/${lanes.coord!.crewId}/${taskId}`)).task.state, {
      timeout: GATE,
      intervals: [5_000],
    })
    .toBe('done');
  await expect.poll(async () => (await textsOf('[data-completed-row]')).includes(title), { timeout: STREAM }).toBe(true);
}

async function workStage(): Promise<void> {
  const a = lanes.a!;
  const b = lanes.b!;
  // A takes the first task; B the second early — its report waits for the first.
  const first = await a.claimNext();
  expect(first.task?.deps_status).toBe('satisfied');
  work.t1 = first.task!.id;
  const second = await b.claimNext();
  expect(second.task?.deps_status).toBe('early_claim_gated');
  work.t2 = second.task!.id;
  // Both in progress in the run's group, each by its lane (#422).
  const active = `${runGroup()} [data-run-stage="active"] [data-run-task-title]`;
  await expect.poll(async () => JSON.parse(await textsOf(active)).sort(), { timeout: STREAM }).toEqual(['First file', 'Second file']);
  // The Working stat counts both lanes on a task (only this crew is open).
  const working = '[data-relay-stat="working"] [data-stat-card-value]';
  await expect.poll(async () => JSON.parse(await textsOf(working))[0], { timeout: STREAM }).toBe('2');

  // A's checkpoint: its steps, counted on its lane row.
  await a.checkpoint(work.t1, {
    current_step: 'Checking the file',
    steps: [
      { key: 'write', title: 'Write t1.txt', state: 'done' },
      { key: 'check', title: 'Check it', state: 'open' },
    ],
  });
  // The holder's first open step reads on its task's row (#422).
  await expect.poll(async () => await textsOf(`${runGroup()} [data-run-task="${work.t1}"] [data-run-task-detail]`), { timeout: STREAM }).toContain('Check it');

  // B's early report is refused until the first is done.
  const early = await b.attempt(() => b.report(work.t2, { phase: 'impl', round: 1, summary: { changed: 't2.txt' }, branch: 'lane/t2' }));
  expect(early).toMatchObject({ ok: false, status: 409, error: 'relay_upstream_deps_unmet' });

  // A reports; the report waits for a reviewer; B claims and approves; the gate merges.
  await a.report(work.t1, { phase: 'impl', round: 1, summary: { changed: 't1.txt' }, branch: 'lane/t1', files: [{ path: 't1.txt', added: 1, removed: 0 }] });
  const floating1 = `[data-floating-review][data-task-id="${work.t1}"]`;
  await expect.poll(() => countOf(floating1), { timeout: STREAM }).toBe(1);
  const review1 = await b.claimNext();
  expect([review1.claim_type, review1.task?.id]).toEqual(['review', work.t1]);
  expect((await b.review(work.t1, 'approved', 'Good.')).state).toBe('approved');
  await expect.poll(() => countOf(floating1), { timeout: STREAM }).toBe(0);
  await mergedAndCompleted(work.t1, 'First file');
  // Done in the order they ran, each by the lane that did it and the one that reviewed it (#422).
  const done = `${runGroup()} [data-run-stage="done"]`;
  await expect.poll(async () => JSON.parse(await textsOf(`${done} [data-run-task-title]`)), { timeout: STREAM }).toEqual(['Plan: Run journey', 'First file']);
  const firstWho = JSON.parse(await textsOf(`${done} [data-run-task="${work.t1}"] [data-run-task-who]`))[0] as string;
  expect(firstWho).toContain('cobalt-e2e');
  expect(firstWho).toContain('reviewed by coral-e2e');
  // By agent: each lane with the run tasks it ran, then what it holds; the choice switches back.
  await pickLaneView('By agent');
  const agentTasks = async (name: string) =>
    JSON.parse(
      String(
        await tauri.evaluate(`JSON.stringify([...document.querySelectorAll('[data-agent-group]')]
          .filter((g) => g.querySelector('[data-agent-name]')?.textContent?.trim() === ${JSON.stringify(name)})
          .flatMap((g) => [...g.querySelectorAll('[data-run-task-title]')].map((t) => t.textContent.trim())))`),
      ),
    ) as string[];
  await expect.poll(() => agentTasks('cobalt-e2e'), { timeout: STREAM }).toEqual(['Plan: Run journey', 'First file']);
  expect(await agentTasks('coral-e2e')).toEqual(['Second file']);
  await pickLaneView('By run');
  await expect.poll(() => countOf(runGroup()), { timeout: STREAM }).toBe(1);
  await evidence('4-work-first-merged');
  execFileSync('git', ['-C', repo, 'merge-base', '--is-ancestor', 'lane/t1', 'main']);

  // Now B reports the second; A reviews; the gate merges.
  await b.report(work.t2, { phase: 'impl', round: 1, summary: { changed: 't2.txt' }, branch: 'lane/t2', files: [{ path: 't2.txt', added: 1, removed: 0 }] });
  const review2 = await a.claimNext();
  expect([review2.claim_type, review2.task?.id]).toEqual(['review', work.t2]);
  expect((await a.review(work.t2, 'approved', 'Good.')).state).toBe('approved');
  await mergedAndCompleted(work.t2, 'Second file');
  execFileSync('git', ['-C', repo, 'merge-base', '--is-ancestor', 'lane/t2', 'main']);
}

async function doneStage(): Promise<void> {
  // Every task done: the broker completes the run on its pass; the band lets it go.
  const band = `[data-sprint-band][data-run-id="${ids.run}"]`;
  await expect.poll(() => countOf(band), { timeout: GATE, intervals: [5_000] }).toBe(0);
  // The Crew view: the run is past — listed under Past runs, opening read-only.
  await view('Crew').click();
  const row = `[data-relay-section="past-runs"] [data-past-run="${ids.run}"]`;
  await expect.poll(() => countOf(row), { timeout: STREAM }).toBe(1);
  expect(await textsOf(row)).toContain('Run journey');
  await evidence('5-done-past-runs');
}
