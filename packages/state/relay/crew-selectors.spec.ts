// The Crew view's selectors (#424; CONTRACT §1h; COMPONENTS §3e): the run card, the
// members grouped by role with their presence, and AddMember's setup or add. Pure: the
// read and a clock in. What the daemon does not report (a heartbeat, the editor a
// session runs in, the catalogue) is not invented (D83).
import { describe, it, expect } from 'vitest';
import { editorTag, toAddMember, toArchivedRun, toMemberGroups, toPastRuns, toRunCard } from './crew-selectors.js';
import { hhmm } from './format.js';
import * as F from './fixtures.js';
import type { RelayCatalogue } from './types.js';

const AT = Date.now();

describe('toRunCard', () => {
  it('a run: its id, running, the repository, when it started, the coordinator, and Members · Tasks · Done', () => {
    const started = F.minutesAgo(120);
    const now = F.now({
      lanes: [F.coordinatorLane({ suggested_name: 'atlas-rigel' }), F.lane()],
      runs: [F.sprint({ state: 'active', started_at: started, total_tasks: 12, done_tasks: 5 })],
    });
    const card = toRunCard(now)!;
    expect(card.title).toBe(`Run ${now.runs[0].run_id.slice(-4)}`);
    expect(card.state).toEqual({ label: 'running', tone: 'success' });
    expect(card.meta).toBe(`sensei · started ${hhmm(started)} · coordinator atlas-rigel · completes, and is archived, when its last assigned task is done`);
    expect(card.facts).toEqual([
      { label: 'Members', value: 2 },
      { label: 'Tasks', value: 12 },
      { label: 'Done', value: 5 },
    ]);
  });

  it('a run still planning reads so', () => {
    const now = F.now({ runs: [F.sprint({ state: 'planning' })] });
    expect(toRunCard(now)!.state).toEqual({ label: 'planning', tone: 'accent' });
  });

  it('a crew with no run yet: the crew by the end of its id, no run yet, its members only', () => {
    const now = F.now({ runs: [], lanes: [F.coordinatorLane()] });
    const card = toRunCard(now)!;
    expect(card.title).toBe(`Crew ${F.CREW_ID.slice(-4)}`);
    expect(card.state).toEqual({ label: 'no run yet', tone: 'neutral' });
    expect(card.facts).toEqual([{ label: 'Members', value: 1 }]);
    expect(card.meta).toContain('coordinator L0');
    expect(card.meta).not.toContain('completes');
  });

  it('no crew, no card; no read, no card', () => {
    expect(toRunCard(F.noCrew())).toBeNull();
    expect(toRunCard(null)).toBeNull();
  });

  it('an open run carries no draft actions', () => {
    const now = F.now({ runs: [F.sprint({ state: 'active' })] });
    expect(toRunCard(now, F.runList())!.draft).toBeNull();
    expect(toRunCard(F.now({ runs: [] }), null)!.draft).toBeNull();
  });
});

describe('toRunCard · a draft waiting to start (#424; #419 draft → start)', () => {
  it('no open run and a draft: the draft, Start once a member is live, and Cancel', () => {
    const now = F.now({ runs: [], lanes: [F.coordinatorLane({ presence: 'idle' })] });
    const card = toRunCard(now, F.runList())!;
    expect(card.title).toBe(`Run ${F.DRAFT_RUN_ID.slice(-4)}`);
    expect(card.state).toEqual({ label: 'draft', tone: 'neutral' });
    expect(card.meta).toBe(`sensei · drafted ${hhmm('2026-11-12T09:00:00Z')} · Sprint 12 · starts on this crew`);
    expect(card.draft).toEqual({ runId: F.DRAFT_RUN_ID, canStart: true, note: null });
  });

  it('a crew with no member answering cannot start it yet, and says so', () => {
    const now = F.now({ runs: [], lanes: [F.coordinatorLane({ presence: 'not_responding' })] });
    expect(toRunCard(now, F.runList())!.draft).toEqual({
      runId: F.DRAFT_RUN_ID,
      canStart: false,
      note: 'Start it once a member is answering.',
    });
  });

  it('no crew yet: the draft still shows; Start waits for the crew', () => {
    const card = toRunCard(F.noCrew(), F.runList())!;
    expect(card.state).toEqual({ label: 'draft', tone: 'neutral' });
    expect(card.facts).toEqual([]);
    expect(card.draft).toEqual({ runId: F.DRAFT_RUN_ID, canStart: false, note: 'Set up your crew to start it.' });
  });

  it('an open run wins over a draft', () => {
    const now = F.now({ runs: [F.sprint({ state: 'planning' })] });
    expect(toRunCard(now, F.runList())!.state.label).toBe('planning');
  });
});

describe('toPastRuns (#424 the UI half of #393; CONTRACT §1i)', () => {
  const at = Date.parse('2026-11-15T12:00:00Z');

  it('one row per ended run, newest first: its id, title, span, tasks done, members, and when it is deleted', () => {
    const list = F.runList({
      archived: [
        F.runListRow({ id: F.PAST_RUN_ID, state: 'completed', title: 'Sprint 11', started_at: '2026-11-01T09:00:00Z', completed_at: '2026-11-02T17:00:00Z', delete_after: '2026-12-02T17:00:00Z', total_tasks: 6, done_tasks: 6, members: 3 }),
        F.runListRow({ id: F.OTHER_RUN_ID, state: 'cancelled', title: 'Sprint 10', started_at: '2026-10-15T09:00:00Z', completed_at: '2026-10-16T09:00:00Z', delete_after: '2026-11-15T09:00:00Z', total_tasks: 4, done_tasks: 1, members: 1 }),
      ],
    });
    expect(toPastRuns(list, at)).toEqual([
      { id: F.PAST_RUN_ID, title: `Run ${F.PAST_RUN_ID.slice(-4)}`, name: 'Sprint 11', state: { label: 'completed', tone: 'success' }, span: 'Nov 1 – Nov 2', done: '6 of 6 tasks done', members: '3 members', deletes: { text: 'deleted in 17 days', warning: false }, href: null },
      { id: F.OTHER_RUN_ID, title: `Run ${F.OTHER_RUN_ID.slice(-4)}`, name: 'Sprint 10', state: { label: 'cancelled', tone: 'neutral' }, span: 'Oct 15 – Oct 16', done: '1 of 4 tasks done', members: '1 member', deletes: { text: 'deleted today', warning: true }, href: null },
    ]);
  });

  it('three days or fewer left reads as a warning', () => {
    const list = F.runList({ archived: [F.runListRow({ id: F.PAST_RUN_ID, state: 'completed', started_at: '2026-10-17T09:00:00Z', completed_at: '2026-10-18T13:00:00Z', delete_after: '2026-11-18T13:00:00Z' })] });
    expect(toPastRuns(list, at)[0].deletes).toEqual({ text: 'deleted in 3 days', warning: true });
  });

  it('a run that never started spans from when it was drafted', () => {
    const list = F.runList({ archived: [F.runListRow({ id: F.PAST_RUN_ID, state: 'cancelled', started_at: null, created_at: '2026-11-03T09:00:00Z', completed_at: '2026-11-03T10:00:00Z', delete_after: '2026-12-03T10:00:00Z' })] });
    expect(toPastRuns(list, at)[0].span).toBe('Nov 3');
  });

  it('with a base, each row opens its run read-only', () => {
    const rows = toPastRuns(F.runList(), at, '/relay');
    expect(rows[0].href).toBe(`/relay/runs/${F.PAST_RUN_ID}`);
  });

  it('none ended, or no read: no rows', () => {
    expect(toPastRuns(F.runList({ archived: [] }), at)).toEqual([]);
    expect(toPastRuns(null, at)).toEqual([]);
  });
});

describe('toMemberGroups · presence (CONTRACT §1h)', () => {
  it('the coordinator, then the contributors with their count', () => {
    const groups = toMemberGroups(F.now({ lanes: [F.coordinatorLane(), F.lane(), F.lane({ id: 'l2', code: 'L2' })] }), AT);
    expect(groups.map((g) => [g.label, g.members.map((m) => m.laneCode)])).toEqual([
      ['Coordinator', ['L0']],
      ['Contributors · 2', ['L1', 'L2']],
    ]);
  });

  it('a member: codename, objective · code, active when on a task, the family and model', () => {
    const [m] = toMemberGroups(F.now({ lanes: [F.lane({ suggested_name: 'cobalt-lyra' })] }), AT)[0].members;
    expect(m).toEqual({
      id: F.LANE.L1,
      codename: 'cobalt-lyra',
      lane: 'Diagrams · L1',
      laneCode: 'L1',
      role: 'contributor',
      status: 'active',
      statusText: 'Active',
      family: 'Claude Code 2.1',
      model: 'claude-opus-5-5',
      alert: null,
      editor: null,
    });
  });

  it('idle with nothing to do; not responding once its lease is lost, with when (D83: no return time invented)', () => {
    const lost = F.minutesAgo(30);
    const [idle, gone] = toMemberGroups(
      F.now({ lanes: [F.lane({ task: null }), F.lane({ id: 'l2', code: 'L2', lease_state: 'expired', lease_expires_at: lost })] }),
      AT,
    )[0].members;
    expect([idle.status, idle.statusText, idle.alert]).toEqual(['idle', 'Idle', null]);
    expect(gone.status).toBe('not_responding');
    expect(gone.statusText).toBe('Not responding');
    expect(gone.alert).toBe(`No answer since ${hhmm(lost)} · lease lost.`);
  });

  it('presence is the daemon’s (#419): a lease past its end, not yet swept, reads not responding', () => {
    const lost = F.minutesAgo(2);
    const [m] = toMemberGroups(
      F.now({ lanes: [F.lane({ lease_state: 'granted', lease_expires_at: lost, presence: 'not_responding' })] }),
      AT,
    )[0].members;
    expect(m.status).toBe('not_responding');
    expect(m.alert).toBe(`No answer since ${hhmm(lost)} · lease lost.`);
  });

  it('a model the session did not report reads so; no codename yet, the code stands in', () => {
    const [m] = toMemberGroups(F.now({ lanes: [F.lane({ model: null })] }), AT)[0].members;
    expect(m.model).toBe('model not reported');
    expect(m.codename).toBe('L1');
  });

  it('no read, no groups', () => {
    expect(toMemberGroups(null, AT)).toEqual([]);
  });
});

describe('toAddMember · set up the crew, or add to it', () => {
  it('no crew: step 1 of 2, the coordinator first, its register line, listening for 0 of 2', () => {
    const add = toAddMember(F.noCrew(), 'coordinator');
    expect(add).toMatchObject({
      mode: 'setup',
      title: 'Start a run: set up your crew',
      role: 'coordinator',
      roleLocked: false,
      command: { step: 'Step 1 of 2.', text: 'register with sensei as coordinator' },
      listening: 'Listening · 0 of 2 to start a run',
    });
  });

  it('a coordinator registered: contributors only, step 2 of 2 while it waits for one', () => {
    const add = toAddMember(F.now({ lanes: [F.coordinatorLane()], runs: [] }), 'coordinator');
    expect(add).toMatchObject({
      mode: 'setup',
      role: 'contributor',
      roleLocked: true,
      // #419: the crew named, so the line works with several crews open.
      command: { step: 'Step 2 of 2.', text: `register with sensei as contributor in crew ${F.CREW_ID}` },
      listening: 'Listening · 1 of 2 to start a run',
    });
  });

  it('a crew working: Add a member, contributors only', () => {
    const add = toAddMember(F.now(), 'contributor');
    expect(add).toMatchObject({ mode: 'add', title: 'Add a member', role: 'contributor', roleLocked: true });
    expect(add.command.step).toBe('');
    expect(add.command.text).toBe(`register with sensei as contributor in crew ${F.CREW_ID}`);
  });

  it('a crew whose coordinator left: the coordinator line names the crew too (#419)', () => {
    const add = toAddMember(F.now({ lanes: [F.lane()], runs: [] }), 'coordinator');
    expect(add.role).toBe('coordinator');
    expect(add.command.text).toBe(`register with sensei as coordinator in crew ${F.CREW_ID}`);
  });
});

describe('editorTag · the host a session runs in (COMPONENTS §3e)', () => {
  it('reads "in <editor>", its title saying whose transcripts the work counts for', () => {
    expect(editorTag('Cursor', 'claude-sonnet-4-5')).toEqual({
      label: 'in Cursor',
      title: 'Cursor is the editor; the assistant inside it does the work, and its transcripts count for claude-sonnet-4-5.',
    });
  });

  it('a lane with no host carries no editor (D83: not invented)', () => {
    const groups = toMemberGroups(F.now());
    expect(groups.flatMap((g) => g.members).every((m) => m.editor === null)).toBe(true);
  });

  it('a lane in an editor names it, by its name, and whose transcripts the work counts for (#396)', () => {
    const now = F.now({ lanes: [F.lane({ host: 'cursor', model: 'claude-sonnet-4-5' })] });
    expect(toMemberGroups(now).flatMap((g) => g.members)[0].editor).toEqual(editorTag('Cursor', 'claude-sonnet-4-5'));
  });

  it('a lane in an editor with no model reported says so in the title', () => {
    const now = F.now({ lanes: [F.lane({ host: 'zed', model: null })] });
    expect(toMemberGroups(now).flatMap((g) => g.members)[0].editor?.title).toContain('model not reported');
  });
});

describe('toArchivedRun (CONTRACT §1i: a past run, read-only)', () => {
  const at = Date.parse('2026-11-15T12:00:00Z');
  const view = () => toArchivedRun(F.runDetail(), { base: '/relay', at });

  it('the run by id and name, its state, the banner: archived when its last task was done; deleted 30 days on, by a daily job', () => {
    const v = view();
    expect(v.title).toBe(`Run ${F.PAST_RUN_ID.slice(-4)}`);
    expect(v.name).toBe('Sprint 11');
    expect(v.state).toEqual({ label: 'completed', tone: 'success' });
    expect(v.banner).toEqual({
      text: 'Archived when its last task was done, Nov 2. Deleted 30 days after it completed, by a daily job: in 17 days.',
      warning: false,
    });
    expect(v.facts).toEqual([
      { label: 'Tasks', value: 3 },
      { label: 'Done', value: 2 },
      { label: 'Members', value: 2 },
    ]);
  });

  it('a cancelled run says so; three days or fewer left reads as a warning', () => {
    const run = F.runListRow({ id: F.PAST_RUN_ID, state: 'cancelled', crew_id: F.CREW_ID, completed_at: '2026-11-15T09:00:00Z', delete_after: '2026-11-17T09:00:00Z' });
    expect(toArchivedRun(F.runDetail({ run }), { base: '/relay', at }).banner).toEqual({
      text: 'Cancelled Nov 15. Deleted 30 days after it ended, by a daily job: in 2 days.',
      warning: true,
    });
  });

  it('who was on it: codename (or code), role, assistant and model', () => {
    expect(view().members).toEqual([
      { id: 'lane-a', codename: 'cobalt-lyra', laneCode: 'L1', role: 'coordinator', family: 'Claude Code', model: 'claude-opus-5-5' },
      { id: 'lane-b', codename: 'L2', laneCode: 'L2', role: 'contributor', family: 'Muse', model: 'model not reported' },
    ]);
  });

  it('what it did: every task, done first, each opening its task view; a task on no lane says so', () => {
    expect(view().tasks).toEqual([
      { id: 't-plan', title: 'Plan: Sprint 11', state: { label: 'done', tone: 'success' }, by: 'cobalt-lyra', href: `/relay/${F.CREW_ID}/t-plan` },
      { id: 't-header', title: 'Diagrams header', state: { label: 'done', tone: 'success' }, by: 'L2', href: `/relay/${F.CREW_ID}/t-header` },
      { id: 't-extra', title: 'Stretch goal', state: { label: 'cancelled', tone: 'neutral' }, by: 'on no lane', href: `/relay/${F.CREW_ID}/t-extra` },
    ]);
  });

  it('a run with no crew (cancelled as a draft) opens no task', () => {
    const run = F.runListRow({ id: F.PAST_RUN_ID, state: 'cancelled', crew_id: null, completed_at: '2026-11-15T09:00:00Z', delete_after: '2026-12-15T09:00:00Z' });
    const v = toArchivedRun(F.runDetail({ run, tasks_by_state: {} }), { base: '/relay', at });
    expect(v.tasks).toEqual([]);
    expect(v.facts[0]).toEqual({ label: 'Tasks', value: 0 });
  });
});

describe('toAddMember · the catalogue picker (#455)', () => {
  const catalogue = (): RelayCatalogue => ({
    assistants: [
      { id: 'claude', name: 'Claude Code', kind: 'assistant', installed: true, last_used_at: F.minutesAgo(120), sessions_week: 3, launch: 'cd "/w" && claude' },
      { id: 'codex', name: 'Codex', kind: 'assistant', installed: null, last_used_at: null, sessions_week: 0, launch: null },
    ],
    editors: [
      { id: 'cursor', name: 'Cursor', kind: 'editor', installed: true, last_used_at: F.minutesAgo(5), models: ['claude-sonnet-4-5', 'gpt-5'], mcp_config: '~/.cursor/mcp.json', launch: null },
      { id: 'zed', name: 'Zed', kind: 'editor', installed: true, last_used_at: null, models: [], mcp_config: null, launch: null },
    ],
    unknown: [{ id: 'cowork', name: 'Cowork', kind: 'unknown', last_used_at: F.minutesAgo(30) }],
  });
  const add = (picked: string | null, data: RelayCatalogue | null = catalogue(), error: string | null = null) =>
    toAddMember(F.now(), 'contributor', { data, error, picked }, AT);
  const DEFAULT_NOTE = `Paste it into an assistant session open in this workspace's repository. It registers, gets a codename, and shows above.`;

  it('two groups: assistants with when they were last used and their sessions this week', () => {
    const view = add(null).catalogue!;
    expect(view.assistants).toEqual([
      { id: 'claude', name: 'Claude Code', used: 'last used 2h ago', details: ['3 sessions this week'], pressed: false, launch: 'cd "/w" && claude' },
      { id: 'codex', name: 'Codex', used: 'not used yet', details: ['no sessions this week'], pressed: false, launch: null },
    ]);
  });

  it('editors with the models seen and where sensei’s tools were found', () => {
    const view = add(null).catalogue!;
    expect(view.editors).toEqual([
      { id: 'cursor', name: 'Cursor', used: 'last used 5 min ago', details: ['claude-sonnet-4-5, gpt-5', "sensei's tools in ~/.cursor/mcp.json"], pressed: false, launch: null },
      { id: 'zed', name: 'Zed', used: 'not used yet', details: ['no models seen', "sensei's tools not set up here"], pressed: false, launch: null },
    ]);
  });

  it('a tool the catalogue does not know is not offered', () => {
    const view = add(null).catalogue!;
    expect([...view.assistants, ...view.editors].map((i) => i.id)).not.toContain('cowork');
  });

  it('one session reads singular', () => {
    const c = catalogue();
    c.assistants[0].sessions_week = 1;
    expect(add(null, c).catalogue!.assistants[0].details).toEqual(['1 session this week']);
  });

  it('nothing picked: today’s note', () => {
    expect(add(null).command.note).toBe(DEFAULT_NOTE);
  });

  it('picking an assistant: pressed, and the note names a session of it in the repository', () => {
    const view = add('claude');
    expect(view.catalogue!.assistants[0].pressed).toBe(true);
    expect(view.command.note).toBe("Paste it into a Claude Code session opened in this workspace's repository.");
  });

  it('picking an editor: the note names its agent chat and where sensei’s tools are', () => {
    const view = add('cursor');
    expect(view.catalogue!.editors[0].pressed).toBe(true);
    expect(view.command.note).toBe("Paste it into Cursor's agent chat. sensei's tools are set up in ~/.cursor/mcp.json.");
    expect(add('zed').command.note).toBe("Paste it into Zed's agent chat. sensei's tools are not set up there yet.");
  });

  it('the paste line is the same whatever is picked', () => {
    const text = add(null).command.text;
    expect(add('cursor').command.text).toBe(text);
    expect(add('claude').command.text).toBe(text);
  });

  it('a read failure is an error, not empty groups', () => {
    const view = add(null, null, 'the daemon is down');
    expect(view.catalogue).toBeNull();
    expect(view.catalogueError).toBe('the daemon is down');
  });

  it('not read (or no catalogue on this surface): no picker, no error', () => {
    const view = toAddMember(F.now(), 'contributor');
    expect(view.catalogue).toBeNull();
    expect(view.catalogueError).toBeNull();
  });
});

describe('toAddMember · the picked tool’s launch (#513)', () => {
  const catalogue = (): RelayCatalogue => ({
    assistants: [
      { id: 'claude', name: 'Claude Code', kind: 'assistant', installed: true, last_used_at: F.minutesAgo(120), sessions_week: 3, launch: 'cd "/w/sensei" && claude' },
      { id: 'codex', name: 'Codex', kind: 'assistant', installed: null, last_used_at: null, sessions_week: 0, launch: null },
    ],
    editors: [
      { id: 'cursor', name: 'Cursor', kind: 'editor', installed: true, last_used_at: F.minutesAgo(5), models: [], mcp_config: null, launch: null },
    ],
    unknown: [],
  });
  const add = (picked: string | null) => toAddMember(F.now(), 'contributor', { data: catalogue(), error: null, picked }, AT);

  it('the picked tool’s resolved launch, for the block under the paste line', () => {
    expect(add('claude').launch).toBe('cd "/w/sensei" && claude');
  });

  it('nothing picked, or a null launch: no block', () => {
    expect(add(null).launch).toBeNull();
    expect(add('codex').launch).toBeNull();
    expect(add('cursor').launch).toBeNull();
  });

  it('items carry their launch for the picker', () => {
    const view = add('claude').catalogue!;
    expect(view.assistants[0].launch).toBe('cd "/w/sensei" && claude');
    expect(view.assistants[1].launch).toBeNull();
  });
});
