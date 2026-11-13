// The Crew view's selectors (#424; CONTRACT §1h; COMPONENTS §3e): the run card, the
// members grouped by role with their presence, and AddMember's setup or add. Pure: the
// read and a clock in. What the daemon does not report (a heartbeat, the editor a
// session runs in, the catalogue) is not invented (D83).
import { describe, it, expect } from 'vitest';
import { toAddMember, toMemberGroups, toRunCard } from './crew-selectors.js';
import { hhmm } from './format.js';
import * as F from './fixtures.js';

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
      command: { step: 'Step 2 of 2.', text: 'register with sensei as contributor' },
      listening: 'Listening · 1 of 2 to start a run',
    });
  });

  it('a crew working: Add a member, contributors only', () => {
    const add = toAddMember(F.now(), 'contributor');
    expect(add).toMatchObject({ mode: 'add', title: 'Add a member', role: 'contributor', roleLocked: true });
    expect(add.command.step).toBe('');
  });
});
