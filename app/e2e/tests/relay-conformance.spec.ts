/**
 * Relay — every state keeps the shell whole (#420; CONTRACT §1a, §1b, §1c-0, §1f;
 * the states of coord/mockups/relay-lanes/Relay.dc.html). Harvested from the
 * conformance lane and rewritten for the r1 shell: the toolbar of icon views,
 * the info band, the stats band on Now, the main window's schema filters.
 *
 * - Error: a real daemon failure, not a seam. The spec moves the `relay` schema
 *   aside in the throwaway `sensei_e2e` DB (checked through /health first), so the
 *   relay read answers 500; Retry after it is put back recovers.
 * - No crew: a workspace of this spec's own, which no crew works in.
 * - Loaded: a crew minted on the e2e broker for a workspace of this run's own,
 *   so other specs' and earlier runs' crews never change what it sees.
 * - Loading is a moment, not a state the harness can hold: the selector and page
 *   specs pin it (relay-shell.spec.ts, the page specs).
 *
 * Each state also writes a DOM dump (and a screenshot when the capture works) to
 * test-results/relay-conformance/ — the report's evidence.
 */
import { execFileSync } from 'child_process';
import { mkdirSync, writeFileSync } from 'fs';
import { test, expect, type TauriPageLike } from '../fixtures';
import { DAEMON_URL, navigateTo, navigateToScreen, seedSetupComplete, seedWorkspace, shot } from '../helpers';
import { psqlBin } from '../lib/psql';

const E2E_DB = 'sensei_e2e';
const EVIDENCE = 'test-results/relay-conformance';
const FRAMING = 'Your crew of assistant sessions';

/** What the screen draws, read in one pass: the report's evidence and the assertions' input. */
interface ShellDump {
  anchor: boolean;
  shell: boolean;
  mark: string;
  title: string;
  context: string;
  /** The title bar's controls, by accessible name with any count dropped. */
  controls: string[];
  pressed: string[];
  info: string | null;
  infoSteps: number;
  todayValue: string;
  stats: Array<{ id: string; text: string }> | null;
  statsReading: boolean;
  filters: { form: boolean; labels: string[]; text: string } | null;
  footer: string[];
  pasteLines: number;
  notice: { text: string; retry: boolean } | null;
  lanes: number;
  completedRows: number;
  decisions: number;
}

async function dump(tauriPage: TauriPageLike): Promise<ShellDump> {
  return (await tauriPage.evaluate(`(function () {
    var root = document.querySelector('[data-screen="relay"]');
    var t = function (el) { return el ? el.textContent.replace(/\\s+/g, ' ').trim() : ''; };
    var q = function (sel) { return root ? root.querySelector(sel) : null; };
    var qa = function (sel) { return root ? Array.prototype.slice.call(root.querySelectorAll(sel)) : []; };
    var header = q('[data-screen-header]');
    var info = q('[data-screen-info]');
    var stats = q('[data-screen-stats]');
    var filters = q('[data-screen-filters]');
    var notice = q('[data-relay-error]');
    var own = function (el) { return !el.closest('[data-screen-info]') && !el.closest('[data-screen-filters]') && !el.closest('[data-screen-stats]'); };
    var name = function (el) { return (el.getAttribute('aria-label') || t(el)).split(' · ')[0]; };
    var controls = header ? Array.prototype.slice.call(header.querySelectorAll('button, a, [role="tab"]')).filter(own) : [];
    return {
      anchor: !!root,
      shell: !!root && root.hasAttribute('data-screen-shell'),
      mark: t(q('[data-screen-mark]')) || (q('[data-screen-mark]') ? 'icon' : ''),
      title: t(q('[data-screen-title]')),
      context: t(q('[data-screen-context]')),
      controls: controls.map(name).sort(),
      pressed: controls.filter(function (el) { return el.getAttribute('aria-pressed') === 'true' || el.getAttribute('data-active') === 'true'; }).map(name).sort(),
      info: info ? t(info) : null,
      infoSteps: info ? info.querySelectorAll('[data-screen-info-step]').length : 0,
      todayValue: (function () {
        var v = info ? info.querySelectorAll('[data-screen-info-value]') : [];
        return v.length ? t(v[v.length - 1]) : '';
      })(),
      stats: stats ? Array.prototype.slice.call(stats.querySelectorAll('[data-stat]')).map(function (el) {
        return { id: el.getAttribute('data-stat'), text: t(el) };
      }) : null,
      statsReading: !!(stats && stats.querySelector('[data-screen-stats-reading]')),
      filters: filters ? {
        form: !!filters.querySelector('[data-form-root]'),
        labels: Array.prototype.slice.call(filters.querySelectorAll('[data-form-field] label, [data-form-field] [data-field-label]')).map(t),
        text: t(filters),
      } : null,
      footer: qa('[data-relay-footer] li').map(t),
      pasteLines: qa('[data-paste-line]').length,
      notice: notice ? { text: t(notice), retry: !!notice.querySelector('[data-action="retry"], button') } : null,
      lanes: qa('[data-lane-row]').length,
      completedRows: qa('[data-completed-row]').length,
      decisions: qa('[data-decision]').length,
    };
  })()`)) as ShellDump;
}

/** The dump and, when the window capture works, a picture — both for the report. */
async function evidence(tauriPage: TauriPageLike, name: string): Promise<ShellDump> {
  const d = await dump(tauriPage);
  mkdirSync(EVIDENCE, { recursive: true });
  writeFileSync(`${EVIDENCE}/${name}.json`, JSON.stringify(d, null, 2));
  // WKWebView paints the page area some time after the DOM settles; a capture taken at
  // once is often unpainted (#284). The wait is for the picture only, never an assertion.
  await new Promise((r) => setTimeout(r, 1200));
  await shot(tauriPage, `${EVIDENCE}/${name}.png`);
  return d;
}

/** The header every state keeps (§1a): the shell is the anchor, its mark, title and context. */
function expectHeader(d: ShellDump, name: string): void {
  expect(d.anchor, `${name}: the relay anchor`).toBe(true);
  expect(d.shell, `${name}: the anchor is the shell`).toBe(true);
  expect(d.mark, `${name}: the mark`).not.toBe('');
  expect(d.title).toBe('Relay');
  expect(d.context, `${name}: the context line`).not.toBe('');
  expect(d.footer.length, `${name}: the footer`).toBeGreaterThan(0);
}

// ─── The error state: the relay schema moved aside in the e2e DB only ────────

function psqlE2e(sql: string): void {
  execFileSync(psqlBin(), ['-d', E2E_DB, '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: 'pipe' });
}

async function breakRelayRead(): Promise<void> {
  // Never on another database: the daemon on the port must be the e2e one.
  const health = (await fetch(`${DAEMON_URL}/health`).then((r) => r.json())) as { dbName?: string };
  if (health.dbName !== E2E_DB) throw new Error(`refusing to move a schema on "${health.dbName}"`);
  psqlE2e('alter schema relay rename to relay_conformance_aside');
}

function restoreRelayRead(): void {
  psqlE2e(
    `do $$ begin
       if exists (select 1 from pg_namespace where nspname = 'relay_conformance_aside') then
         alter schema relay_conformance_aside rename to relay;
       end if;
     end $$`,
  );
}

async function relayCall<T>(method: 'GET' | 'POST', path: string, token: string | null, body?: unknown): Promise<T> {
  const resp = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  if (!resp.ok) throw new Error(`${method} ${path}: HTTP ${resp.status}: ${await resp.text()}`);
  return (await resp.json()) as T;
}

interface Lane {
  crew_id: string;
  lane_id: string;
  lane_token: string;
}

/** A task planned, assigned and delivered: the holder's envelope pull flips it to working. */
async function startTask(coord: Lane, holder: Lane, reviewer: Lane, title: string): Promise<string> {
  const { task_id: task } = await relayCall<{ task_id: string }>('POST', '/api/relay/tasks/plan', coord.lane_token, {
    lane_id: coord.lane_id,
    title,
    phase: 'impl',
    repo_key: 'sensei',
  });
  await relayCall('POST', `/api/relay/tasks/${task}/assign`, coord.lane_token, {
    lane_id: coord.lane_id,
    to_lane: holder.lane_id,
    title,
    phase: 'impl',
    brief: { summary: 'Relay conformance e2e task' },
    reviewer_lane: reviewer.lane_id,
  });
  await relayCall('GET', `/api/relay/tasks?lane_id=${holder.lane_id}`, holder.lane_token);
  return task;
}

/** One task the whole way to done on the broker (the F8 choreography: plan → assign →
 *  envelope pull → report → claim the review → approve: done), so Today and Completed
 *  today have a row. */
async function completeOneTask(coord: Lane, holder: Lane, reviewer: Lane, title: string): Promise<void> {
  const task = await startTask(coord, holder, reviewer, title);
  await relayCall('POST', `/api/relay/tasks/${task}/report`, holder.lane_token, {
    lane_id: holder.lane_id,
    phase: 'impl',
    round: 1,
    summary: { changed: 'Done for the conformance pin' },
  });
  // The report floats on the review queue; a lane claims it before it may review it.
  await relayCall('POST', `/api/relay/tasks/${task}/claim_review`, reviewer.lane_token, { lane_id: reviewer.lane_id });
  await relayCall('POST', `/api/relay/tasks/${task}/review`, reviewer.lane_token, {
    lane_id: reviewer.lane_id,
    verdict: 'approved',
    summary: 'Approved for the conformance pin',
  });
  // With no sprint gate to merge through, the broker finishes an approved task itself.
}

/** The daemon converges the relay schema after boot, in the background; a relay call
 *  before that is a 500 that is the boot's, not the screen's. Wait for the read to answer. */
async function relayConverged(budgetMs = 240_000): Promise<void> {
  const deadline = Date.now() + budgetMs;
  let last = '';
  while (Date.now() < deadline) {
    const resp = await fetch(`${DAEMON_URL}/relay/now`).catch((e: unknown) => {
      last = String(e);
      return null;
    });
    if (resp?.ok) return;
    if (resp) last = `HTTP ${resp.status}: ${await resp.text()}`;
    await new Promise((r) => setTimeout(r, 2_000));
  }
  throw new Error(`the relay schema never converged on the e2e daemon: ${last}`);
}

test.describe('Relay — every state keeps the shell whole', () => {
  test.beforeAll(async () => {
    test.setTimeout(300_000);
    await relayConverged();
  });

  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target
  });

  test.afterEach(() => restoreRelayRead());

  test('error: the header, the whole info band, only the info toggle, and the §1f Notice — Retry recovers', async ({
    tauriPage,
  }) => {
    test.setTimeout(180_000);
    const ws = await seedWorkspace('Relay conformance · error', 'relay-conformance e2e: the error state');
    await breakRelayRead();

    for (const [name, path] of [
      ['error-main', '/relay'],
      ['error-workspace', `/workspace/${ws}/relay`],
    ] as const) {
      await navigateToScreen(tauriPage, path, '[data-relay-error]');
      const d = await evidence(tauriPage, name);
      expectHeader(d, name);
      // Nothing was read: no views to switch, nobody to message (§1f).
      expect(d.controls, `${name}: the toolbar`).toEqual(['Hide info']);
      expect(d.context, `${name}: never a count it did not read (D83)`).toMatch(/· crew not reported$/);
      expect(d.info, `${name}: the info band`).toContain(FRAMING);
      expect(d.infoSteps, `${name}: the three steps`).toBe(3);
      expect(d.todayValue, `${name}: Today`).toBe('not reported');
      expect(d.stats, `${name}: no stats band`).toBeNull();
      expect(d.pasteLines, `${name}: not the no-crew setup`).toBe(0);
      expect(d.notice, `${name}: the Notice`).not.toBeNull();
      expect(d.notice!.text).toContain("Couldn't read the crew");
      expect(d.notice!.retry).toBe(true);
    }

    restoreRelayRead();
    await tauriPage.click('[data-relay-error] [data-action="retry"]');
    await tauriPage.waitForSelector('[data-screen="relay"] [data-paste-line]', 15_000);
    const after = await dump(tauriPage);
    expect(after.notice, 'Retry replaced the Notice').toBeNull();
  });

  test('no crew: Crew is the landing, the band says what to do, held open, the paste lines below', async ({
    tauriPage,
  }) => {
    test.setTimeout(180_000);
    const ws = await seedWorkspace('Relay conformance · no crew', 'relay-conformance e2e: no crew works here');
    await navigateToScreen(tauriPage, `/workspace/${ws}/relay`, '[data-screen="relay"] [data-paste-line]');
    const d = await evidence(tauriPage, 'no-crew-workspace');
    expectHeader(d, 'no-crew');
    // The views stay; there is nobody to message yet.
    expect(d.controls).toEqual(['Crew', 'Hide info', 'Now', 'Queue']);
    expect(d.pressed).toContain('Crew');
    expect(d.context).toMatch(/· no run$/);
    expect(d.info).toContain(FRAMING);
    expect(d.info).toContain('No crew yet');
    expect(d.stats, 'no stats band without a crew').toBeNull();
    expect(d.pasteLines).toBe(2);
    expect(d.notice).toBeNull();
    expect(d.lanes).toBe(0);
  });

  test('loaded: a crew minted on the broker under the whole header — views, stats, filters — in both windows', async ({
    tauriPage,
  }) => {
    test.setTimeout(180_000);
    // A workspace of this run's own: the e2e DB can outlive a run (reset=false), and a
    // crew minted by an earlier run must not add its lanes to this one's count.
    const ws = await seedWorkspace(
      `Relay conformance · loaded · ${Date.now()}`,
      'relay-conformance e2e: one crew of four lanes, a task done today, one on a lane, a decision',
    );
    const coord = await relayCall<Lane>('POST', '/api/relay/register', null, {
      role: 'coordinator',
      family: 'e2e',
      goal: 'relay-conformance loaded crew',
      workspace_id: ws,
    });
    const contributor = () =>
      relayCall<Lane>('POST', '/api/relay/register', null, { role: 'contributor', family: 'e2e', crew_id: coord.crew_id });
    const holder = await contributor();
    const reviewer = await contributor();
    const worker = await contributor();
    await completeOneTask(coord, holder, reviewer, 'Relay conformance · finished today');
    // A lane on a task, and a decision that needs the person (§1c-1, §1c-2).
    const working = await startTask(coord, worker, reviewer, 'Relay conformance · on a lane now');
    await relayCall('POST', '/api/relay/decisions', coord.lane_token, {
      lane_id: coord.lane_id,
      task_id: working,
      question: 'Where does the conformance log live?',
      context: 'The lane on a task is waiting on this.',
      options: [
        { id: 'db', label: "In the daemon's database" },
        { id: 'file', label: 'In a file per workspace' },
      ],
      recommended: 'db',
      asked_of: 'person',
    });

    for (const [name, path, main] of [
      ['loaded-workspace', `/workspace/${ws}/relay`, false],
      ['loaded-main', `/relay?workspace=${ws}`, true],
    ] as const) {
      await navigateToScreen(tauriPage, path, '[data-screen="relay"] [data-lane-row]');
      const d = await evidence(tauriPage, name);
      expectHeader(d, name);
      expect(d.controls, `${name}: the three views, Message the crew, the info toggle`).toEqual([
        'Crew',
        'Hide info',
        'Message the crew',
        'Now',
        'Queue',
      ]);
      expect(d.pressed, `${name}: Now is pressed`).toContain('Now');
      expect(d.context, `${name}: the crew by the end of its id`).toMatch(new RegExp(`· crew ${coord.crew_id.slice(-4)}$`));
      if (main) expect(d.context).toMatch(/^all workspaces ·/);
      expect(d.info).toContain(FRAMING);
      expect(d.infoSteps).toBe(3);
      expect(d.todayValue, `${name}: Today is a number`).toMatch(/^[1-9]\d*$/);
      // §1c-0: the four cards, counted from the read.
      expect(d.stats?.map((s) => s.id), `${name}: the stats band`).toEqual(['working', 'waiting', 'blocked', 'done']);
      const value = (id: string) => Number((d.stats!.find((s) => s.id === id)!.text.match(/\d+/) ?? ['NaN'])[0]);
      expect(value('working'), `${name}: the lane on a task`).toBeGreaterThanOrEqual(1);
      expect(value('blocked'), `${name}: the decision`).toBeGreaterThanOrEqual(1);
      expect(value('done'), `${name}: the finished task`).toBeGreaterThanOrEqual(1);
      // #384: the main window narrows through a rokkit schema form; the workspace window has none.
      if (main) {
        expect(d.filters?.form, `${name}: the schema form`).toBe(true);
        expect(d.filters?.text).toContain('Repository');
        expect(d.filters?.text).toContain('Workspace');
      } else {
        expect(d.filters, `${name}: no filter band`).toBeNull();
      }
      expect(d.notice).toBeNull();
      expect(d.lanes, `${name}: the four lanes`).toBe(4);
      expect(d.completedRows, `${name}: the finished task`).toBeGreaterThanOrEqual(1);
      expect(d.decisions, `${name}: the decision in Needs you`).toBeGreaterThanOrEqual(1);
    }
  });
});
