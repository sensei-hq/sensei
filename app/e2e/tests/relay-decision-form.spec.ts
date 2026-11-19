/**
 * A relay decision answered with a form (#389), against the e2e broker.
 *
 * Slice A — the wire: a lane asks the person with a `response` (a flat JSON Schema
 * subset); `GET /relay/now` lists it with that response and no options; the daemon
 * refuses values that break the schema (422 naming the field, nothing recorded); the
 * person answers with valid values; the asking lane's envelope reads them back
 * exactly, with `option` null. The option-pick journey stays in relay-run.spec.ts.
 *
 * The task view reads a form answer back as its values, in the timeline row and its
 * expanded entry — never "null" or an empty option.
 *
 * Slice B — the screen: a form decision arrives in Needs you while the Relay screen is
 * open, drawn as one control per property (`data-decision-field`); Answer with the
 * required field empty shows its message and sends nothing; filled and answered, the
 * card leaves with no navigation and the lane reads the exact values; the task view
 * answers a task-bound form the same way. "No refresh" is proved by a marker on
 * `window` that a reload would wipe, and an unchanged path.
 *
 * Slice R (R2) — pick-several and long text: rokkit 1.16 draws an array of enum members as a
 * MultiSelect and an `x-multiline` string as a textarea; the person answers both live and the
 * lane reads the exact members and text back.
 */
import { mkdirSync, readFileSync, writeFileSync } from 'fs';
import { test, expect, type TauriPageLike } from '../fixtures';
import { DAEMON_URL, navigateTo, navigateToScreen } from '../helpers';
import { RelayCrew, type RelayLane } from '../lib/relay-crew';

const RUN_TAG = Date.now().toString(36);

interface DecisionRow {
  id: string;
  options: unknown[];
  response: unknown;
}

/** What the routes this spec reads answer with (only the fields it checks). */
interface Answered {
  decisions: DecisionRow[];
  message: string;
}

/** A person route on the daemon (no lane token): its status and body, refusals included. */
async function person(method: string, path: string, body?: unknown): Promise<{ status: number; body: Answered }> {
  const res = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  return { status: res.status, body: text ? JSON.parse(text) : null };
}

const RESPONSE = {
  schema: {
    type: 'object',
    properties: {
      target: { type: 'string', enum: ['develop', 'main'], title: 'Target branch' },
      count: { type: 'integer', minimum: 1, maximum: 5 },
      ship: { type: 'boolean', description: 'Ship it now?' },
    },
    required: ['target'],
  },
};
/** The layout the daemon stores for RESPONSE: its properties in the order the lane wrote them. */
const LANE_ORDER = {
  type: 'vertical',
  elements: [{ scope: '#/target' }, { scope: '#/count' }, { scope: '#/ship' }],
};

test.describe('relay decision form (#389)', () => {
  let lane: RelayLane | null = null;

  test.afterAll(async () => {
    if (!lane) return;
    await person('POST', `/relay/crews/${lane.crewId}/close`, {});
  });

  test('the broker round-trips a form decision', async () => {
    lane = await new RelayCrew(DAEMON_URL).register({
      role: 'coordinator',
      family: 'e2e',
      goal: `decision form ${RUN_TAG}`,
      repo_key: 'github.com/sensei-hq/e2e-form',
    });
    const decisionId = await lane.ask({ question: `Where does it ship? ${RUN_TAG}`, response: RESPONSE });

    // Now lists it with its response and no buttons.
    const now = await person('GET', '/relay/now');
    expect(now.status).toBe(200);
    const row = now.body.decisions.find((d) => d.id === decisionId);
    expect(row, 'the form decision is on Now').toBeTruthy();
    // The schema as sent; with no layout sent, the daemon stores one in the lane's order.
    expect(row?.response).toEqual({ ...RESPONSE, layout: LANE_ORDER });
    expect(row?.options).toEqual([]);

    // Values that break the schema are refused by name, and nothing is recorded.
    const bad = await person('POST', '/relay/answers', {
      decision_id: decisionId,
      values: { target: 'main', count: 9 },
      note: null,
      surface: 'desktop',
    });
    expect(bad.status).toBe(422);
    expect(bad.body.message).toContain('values.count');
    const missing = await person('POST', '/relay/answers', { decision_id: decisionId, values: { ship: true }, note: null, surface: 'desktop' });
    expect(missing.status).toBe(422);
    expect(missing.body.message).toContain('values.target is required');
    expect((await lane.envelope()).answers.filter((a) => a.decision_id === decisionId)).toEqual([]);

    // The person answers; the asking lane reads the exact values, option null.
    const values = { target: 'main', count: 3, ship: true };
    const ok = await person('POST', '/relay/answers', { decision_id: decisionId, values, note: null, surface: 'desktop' });
    expect(ok.status, JSON.stringify(ok.body)).toBe(200);
    const answer = (await lane.envelope()).answers.find((a) => a.decision_id === decisionId);
    expect(answer).toMatchObject({ decision_id: decisionId, option: null, values });

    // Answered: it leaves Now.
    const after = await person('GET', '/relay/now');
    expect(after.body.decisions.some((d) => d.id === decisionId)).toBe(false);
  });

  test('the task view reads a form answer as its values', async ({ tauriPage }) => {
    test.setTimeout(120_000);
    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ setup_complete: '1' }),
    });
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target

    const coord = await new RelayCrew(DAEMON_URL).register({
      role: 'coordinator',
      family: 'e2e',
      goal: `decision form task ${RUN_TAG}`,
      repo_key: 'github.com/sensei-hq/e2e-form',
    });
    lane ??= coord;
    const title = `Form answer ${RUN_TAG}`;
    const taskId = await coord.plan({ title, phase: 'impl', repo_key: 'github.com/sensei-hq/e2e-form' });
    const decisionId = await coord.ask({ task_id: taskId, question: `Where does it ship? ${RUN_TAG}`, response: RESPONSE });
    const values = { target: 'main', count: 3, ship: true };
    const ok = await person('POST', '/relay/answers', { decision_id: decisionId, values, note: null, surface: 'desktop' });
    expect(ok.status, JSON.stringify(ok.body)).toBe(200);

    await navigateToScreen(tauriPage, `/relay/${coord.crewId}/${taskId}`, '[data-screen="relay"] [data-task-title]');
    await expect(tauriPage.locator('[data-task-title]')).toContainText(title);
    const row = tauriPage.locator('[data-timeline-toggle]').filter({ hasText: 'Answered:' });
    await expect(row).toContainText('Answered: count: 3, ship: true, target: main');
    await row.click();
    await expect(tauriPage.locator('[data-relay-timeline] [data-timeline-detail] [data-entry-answer]')).toContainText(
      'count: 3, ship: true, target: main · from the desktop',
    );

    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });

  // ── slice B: the form on the screen, live ────────────────────────────────────

  const STREAM = 20_000;
  const EVIDENCE = 'test-results/relay-decision-form';
  const NEEDS = '[data-relay-section="needs-you"]';
  const FORM_REPO = 'github.com/sensei-hq/e2e-form-screen';

  /** How many elements a selector matches, counted in the page (this driver has no count). */
  const countOf = async (page: TauriPageLike, selector: string) =>
    Number(await page.evaluate(`document.querySelectorAll(${JSON.stringify(selector)}).length`));
  /** The text of every element a selector matches. */
  const textsOf = async (page: TauriPageLike, selector: string): Promise<string[]> =>
    JSON.parse(
      (await page.evaluate(
        `JSON.stringify([...document.querySelectorAll(${JSON.stringify(selector)})].map((e) => e.textContent.replace(/\\s+/g, ' ').trim()))`,
      )) as string,
    );
  /** Mark the page: a reload or a remount of the document wipes it. */
  const markPage = (page: TauriPageLike) => page.evaluate(`(window.__formNoReload = ${JSON.stringify(RUN_TAG)}, location.pathname)`) as Promise<string>;
  const stillMarked = (page: TauriPageLike) =>
    page.evaluate(`JSON.stringify({ mark: window.__formNoReload ?? null, at: location.pathname })`) as Promise<string>;
  /** Pick a value in a field's rokkit Select: open it, press the option by its text. */
  async function pick(page: TauriPageLike, card: string, name: string, label: string): Promise<void> {
    // Scrolled into view first: the task view's card can sit below the fold, where a pointer
    // click misses. The Select opens on a real pointer press, not a scripted click.
    const trigger = `${card} [data-decision-field="${name}"] [data-select-trigger]`;
    await page.evaluate(`(document.querySelector(${JSON.stringify(trigger)}).scrollIntoView({ block: 'center' }), true)`);
    await page.locator(trigger).click();
    await expect
      .poll(() =>
        page.evaluate(`(() => {
          const o = [...document.querySelectorAll(${JSON.stringify(`${card} [data-decision-field="${name}"] [data-select-option]`)})]
            .find((e) => e.textContent.trim() === ${JSON.stringify(label)});
          if (!o) return false;
          o.click();
          return true;
        })()`),
      )
      .toBe(true);
    await expect.poll(async () => (await textsOf(page, `${card} [data-decision-field="${name}"] [data-select-value-text]`))[0] ?? '').toBe(label);
  }
  /** Set a number field as typing would: input, then change. */
  const setNumber = (page: TauriPageLike, card: string, name: string, value: string) =>
    page.evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(`${card} [data-decision-field="${name}"] input[type="number"]`)});
      if (!el) throw new Error('no number input for ${name}');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(el, ${JSON.stringify(value)});
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
  /** The card's DOM, kept as evidence beside the run (the window capture is not reliable here). */
  async function evidence(page: TauriPageLike, name: string, selector: string): Promise<void> {
    mkdirSync(EVIDENCE, { recursive: true });
    const html = (await page.evaluate(`document.querySelector(${JSON.stringify(selector)})?.outerHTML ?? '(gone)'`)) as string;
    writeFileSync(`${EVIDENCE}/${name}.html`, html);
  }

  async function openRelay(page: TauriPageLike): Promise<void> {
    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ setup_complete: '1' }),
    });
    await navigateTo(page, '/logs');
    await navigateToScreen(page, '/relay', '[data-screen="relay"] [data-screen-tools]');
    await page.locator('[data-screen-tools] [aria-label^="Now"]').click();
    await page.waitForSelector(NEEDS, 20_000).catch(() => undefined);
  }

  /** The screen tests share one coordinator and one open screen session per test. */
  async function formLane(goal: string): Promise<RelayLane> {
    return new RelayCrew(DAEMON_URL).register({ role: 'coordinator', family: 'e2e', goal: `${goal} ${RUN_TAG}`, repo_key: FORM_REPO });
  }

  test('a form decision arrives with no refresh', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await openRelay(tauriPage);
    const at = await markPage(tauriPage);
    const coord = await formLane('form arrives');
    lane ??= coord;
    const question = `Arrives live? ${RUN_TAG}`;
    const decisionId = await coord.ask({ question, response: RESPONSE });
    const card = `${NEEDS} [data-decision="${decisionId}"]`;

    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(1);
    expect(await textsOf(tauriPage, `${card} [data-decision-question]`)).toEqual([question]);
    expect(await textsOf(tauriPage, `${card} [data-decision-field]`).then((t) => t.length)).toBe(3);
    // The fields come in the order the lane wrote them, though jsonb stores the keys shorter-first.
    const names = JSON.parse(
      (await tauriPage.evaluate(
        `JSON.stringify([...document.querySelectorAll(${JSON.stringify(`${card} [data-decision-field]`)})].map((e) => e.getAttribute('data-decision-field')))`,
      )) as string,
    );
    expect(names).toEqual(['target', 'count', 'ship']);
    expect(await countOf(tauriPage, `${card} [data-decision-field="target"] [data-select]`)).toBe(1);
    expect(await countOf(tauriPage, `${card} [data-decision-field="count"] input[type="number"]`)).toBe(1);
    expect(await countOf(tauriPage, `${card} [data-decision-field="ship"] input[type="checkbox"]`)).toBe(1);
    expect(await countOf(tauriPage, `${card} [data-option]`)).toBe(0);
    expect(JSON.parse(await stillMarked(tauriPage))).toEqual({ mark: RUN_TAG, at });
    await evidence(tauriPage, '1-arrived', card);

    // The card passes axe with the form showing (WCAG A + AA; no serious or critical).
    await tauriPage.evaluate(
      `(function () { if (!window.axe) { ${readFileSync('node_modules/axe-core/axe.min.js', 'utf8')} } return typeof window.axe === 'object'; })()`,
    );
    const violations = JSON.parse(
      (await tauriPage.evaluate(`(async function () {
        var r = await window.axe.run(document.querySelector(${JSON.stringify(card)}), {
          runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] },
          resultTypes: ['violations'],
        });
        return JSON.stringify((r.violations || [])
          .filter(function (v) { return v.impact === 'serious' || v.impact === 'critical'; })
          .map(function (v) { return v.id + ': ' + v.nodes.map(function (n) { return n.target.join(' '); }).join(', '); }));
      })()`)) as string,
    );
    expect(violations, 'serious/critical a11y violations on the form card').toEqual([]);
    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });

  test('required fields refuse inline', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await openRelay(tauriPage);
    const coord = await formLane('form refuses');
    lane ??= coord;
    const decisionId = await coord.ask({ question: `Refuse inline? ${RUN_TAG}`, response: RESPONSE });
    const card = `${NEEDS} [data-decision="${decisionId}"]`;
    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(1);

    // Every answer the page posts is counted: the inline refusal must send none, which a
    // daemon 422 (the card stays, the envelope stays empty) could not tell apart.
    await tauriPage.evaluate(`(() => {
      window.__answerPosts = 0;
      if (!window.__answerSpy) {
        const real = window.fetch;
        window.fetch = (input, init) => {
          const url = typeof input === 'string' ? input : input.url;
          if (url.includes('/relay/answers')) window.__answerPosts += 1;
          return real(input, init);
        };
        window.__answerSpy = true;
      }
      return true;
    })()`);
    await tauriPage.locator(`${card} [data-decision-answer]`).click();
    await expect
      .poll(async () => (await textsOf(tauriPage, `${card} [data-decision-field="target"] [data-message]`))[0] ?? '', { timeout: STREAM })
      .toContain('is required');
    expect(Number(await tauriPage.evaluate('window.__answerPosts'))).toBe(0);
    expect(await countOf(tauriPage, card)).toBe(1);
    expect((await coord.envelope()).answers.filter((a) => a.decision_id === decisionId)).toEqual([]);
    await evidence(tauriPage, '2-refused-inline', card);
    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });

  test('answering resolves with no refresh', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await openRelay(tauriPage);
    const at = await markPage(tauriPage);
    const coord = await formLane('form resolves');
    lane ??= coord;
    const decisionId = await coord.ask({ question: `Resolve live? ${RUN_TAG}`, response: RESPONSE });
    const card = `${NEEDS} [data-decision="${decisionId}"]`;
    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(1);

    await pick(tauriPage, card, 'target', 'main');
    await setNumber(tauriPage, card, 'count', '3');
    await evidence(tauriPage, '3-filled', card);
    await tauriPage.locator(`${card} [data-decision-answer]`).click();

    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(0);
    expect(JSON.parse(await stillMarked(tauriPage))).toEqual({ mark: RUN_TAG, at });
    // ship was never touched: absent, not false (D83).
    const answer = (await coord.envelope()).answers.find((a) => a.decision_id === decisionId);
    expect(answer).toMatchObject({ decision_id: decisionId, option: null, values: { target: 'main', count: 3 } });
    expect(Object.keys((answer as { values: Record<string, unknown> }).values).sort()).toEqual(['count', 'target']);
    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });

  test('a pick-several and long-text decision is answered live', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await openRelay(tauriPage);
    const at = await markPage(tauriPage);
    const coord = await formLane('form several');
    lane ??= coord;
    const several = {
      schema: {
        type: 'object',
        properties: {
          areas: { type: 'array', items: { type: 'string', enum: ['api', 'app', 'db'] }, uniqueItems: true, title: 'Areas' },
          notes: { type: 'string', 'x-multiline': true, maxLength: 2000, title: 'Notes' },
        },
        required: ['areas'],
      },
    };
    const decisionId = await coord.ask({ question: `Pick several? ${RUN_TAG}`, response: several });
    const card = `${NEEDS} [data-decision="${decisionId}"]`;
    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(1);
    expect(await countOf(tauriPage, `${card} [data-decision-field="areas"] [data-multiselect]`)).toBe(1);
    expect(await countOf(tauriPage, `${card} [data-decision-field="notes"] textarea`)).toBe(1);

    // Pick two members: open the MultiSelect with a real press, then press each option.
    const trigger = `${card} [data-decision-field="areas"] [data-select-trigger]`;
    await tauriPage.evaluate(`(document.querySelector(${JSON.stringify(trigger)}).scrollIntoView({ block: 'center' }), true)`);
    await tauriPage.locator(trigger).click();
    for (const member of ['db', 'api']) {
      await expect
        .poll(() =>
          tauriPage.evaluate(`(() => {
            const o = [...document.querySelectorAll(${JSON.stringify(`${card} [data-decision-field="areas"] [data-select-option]`)})]
              .find((e) => e.textContent.trim() === ${JSON.stringify(member)});
            if (!o) return false;
            o.click();
            return true;
          })()`),
        )
        .toBe(true);
    }
    const notes = 'first line\nsecond line';
    await tauriPage.evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(`${card} [data-decision-field="notes"] textarea`)});
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(el, ${JSON.stringify(notes)});
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
    await evidence(tauriPage, '5-several-filled', card);
    await tauriPage.locator(`${card} [data-decision-answer]`).click();

    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(0);
    expect(JSON.parse(await stillMarked(tauriPage))).toEqual({ mark: RUN_TAG, at });
    const answer = (await coord.envelope()).answers.find((a) => a.decision_id === decisionId) as
      | { option: unknown; values: { areas: string[]; notes: string } }
      | undefined;
    expect(answer?.option).toBeNull();
    expect([...(answer?.values.areas ?? [])].sort()).toEqual(['api', 'db']);
    expect(answer?.values.notes).toBe(notes);
    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });

  test('the task view answers too', async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ setup_complete: '1' }),
    });
    await navigateTo(tauriPage, '/logs');
    const coord = await formLane('form task view');
    lane ??= coord;
    const title = `Form on the task ${RUN_TAG}`;
    const taskId = await coord.plan({ title, phase: 'impl', repo_key: FORM_REPO });
    await navigateToScreen(tauriPage, `/relay/${coord.crewId}/${taskId}`, '[data-screen="relay"] [data-task-title]');
    const at = await markPage(tauriPage);

    // Asked while the task view is open: it arrives there too.
    const decisionId = await coord.ask({ task_id: taskId, question: `Answer from the task? ${RUN_TAG}`, response: RESPONSE });
    const card = `[data-decision="${decisionId}"]`;
    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(1);
    await pick(tauriPage, card, 'target', 'develop');
    await tauriPage.locator(`${card} [data-decision-answer]`).click();

    await expect.poll(() => countOf(tauriPage, card), { timeout: STREAM }).toBe(0);
    expect(JSON.parse(await stillMarked(tauriPage))).toEqual({ mark: RUN_TAG, at });
    const answer = (await coord.envelope()).answers.find((a) => a.decision_id === decisionId);
    expect(answer).toMatchObject({ option: null, values: { target: 'develop' } });
    await evidence(tauriPage, '4-task-view-answered', '[data-screen="relay"]');
    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });
});
