// The task view's selectors (#425; CONTRACT §1e; COMPONENTS §3d TaskHeader, RunDetails,
// HandoffCard): from the task's read to what each part draws. A number nobody reported
// reads so, never 0 (D83).
import { describe, it, expect } from 'vitest';
import { toTaskView } from './task-selectors.js';
import { hhmm } from './format.js';
import * as F from './fixtures.js';

const opts = { back: { label: 'Now', href: '/relay' } };

describe('toTaskView · the header', () => {
  it('back, the title, its state, the ticket, and the facts: Lane, Assistant, Time, Rounds', () => {
    const view = F.taskView();
    const v = toTaskView(view, opts);
    expect(v.header.back).toEqual({ label: 'Now', href: '/relay' });
    expect(v.header.title).toBe(view.task.title);
    // The header reads the task's state; it carries no merge outcome to say "merged".
    expect(v.header.state).toEqual({ label: 'done', tone: 'success' });
    expect(v.header.facts.map((f) => f.k)).toEqual(['Lane', 'Assistant', 'Time', 'Rounds']);
    const lane = view.task.lane!;
    expect(v.header.facts[0].v).toBe(`${lane.title} · ${lane.code}`);
    expect(v.header.facts[1].v).toBe(`Claude Code ${lane.version} · ${lane.model}`);
  });

  it('Time spans assigned to completed, with how long; a working task reads in the accent tone', () => {
    const assigned = '2026-11-12T06:00:00Z';
    const completed = '2026-11-12T10:02:00Z';
    const view = F.taskView({ task: { ...F.taskView().task, state: 'done', assigned_at: assigned, completed_at: completed } });
    const v = toTaskView(view, opts);
    expect(v.header.facts[2].v).toBe(`${hhmm(assigned)} → ${hhmm(completed)} · 4 h 2 min`);
    const working = toTaskView(F.taskView({ task: { ...F.taskView().task, state: 'working', completed_at: null } }), opts);
    expect(working.header.state).toEqual({ label: 'working', tone: 'accent' });
  });

  it('no lane: the task waits in the pool; an assistant nobody reported reads so', () => {
    const v = toTaskView(F.taskView({ task: { ...F.taskView().task, lane: null, assigned_at: null } }), opts);
    expect(v.header.facts[0].v).toBe('in the pool');
    expect(v.header.facts[1].v).toBe('not reported');
    expect(v.header.facts[2].v).toMatch(/^unclaimed/);
  });

  it('a holder paused on a limit: the Lane fact says which quota and until when (or that it is unknown)', () => {
    const resumes = '2026-11-12T14:00:00Z';
    const lane = { ...F.taskView().task.lane!, limit_scope: 'weekly' as const, limit_resumes_at: resumes };
    expect(toTaskView(F.taskView({ task: { ...F.taskView().task, lane } }), opts).header.facts[0].v).toBe(
      `Diagrams · L1 · paused · weekly · until ${hhmm(resumes)}`,
    );
    const unknown = { ...lane, limit_scope: 'daily' as const, limit_resumes_at: null };
    expect(toTaskView(F.taskView({ task: { ...F.taskView().task, lane: unknown } }), opts).header.facts[0].v).toBe(
      'Diagrams · L1 · paused · daily · resume unknown',
    );
  });

  it('the ticket: provider, key and link', () => {
    const view = F.taskView({ task: { ...F.taskView().task, ticket: { provider: 'jira', key: 'SEN-52', url: 'https://x/SEN-52' } } });
    expect(toTaskView(view, opts).header.ticket).toEqual({ provider: 'Jira', key: 'SEN-52', url: 'https://x/SEN-52' });
  });
});

describe('toTaskView · Run details', () => {
  it('a one-line summary, and every row in the contract’s order; a value nobody reported is null', () => {
    const view = F.taskView({
      run: { ...F.run(), tokens_in: 3_100_000, working_secs: 8520, subagents: 1, peak_memory_mb: null },
    });
    const run = toTaskView(view, opts).run;
    expect(run.line).toBe('3.1 M tokens in · 2 h 22 min working · 1 subagent');
    expect(run.rows.map((r) => r.k)).toEqual([
      'Repository',
      'Branch',
      'Commits',
      'Model',
      'Tokens in',
      'Tokens out',
      'Tokens cached',
      'Peak context',
      'Compactions',
      'Peak memory',
      'Subagents',
      'Working',
      'Waiting for review',
      'Files changed',
      'Gates run',
    ]);
    expect(run.rows.find((r) => r.k === 'Peak memory')!.v).toBeNull();
    expect(run.note).toContain(`of ${view.run.turns} reported`);
  });

  it('nothing reported: the line says so, never "0 tokens"', () => {
    const view = F.taskView({ run: { ...F.run(), tokens_in: null, working_secs: null, subagents: null } });
    expect(toTaskView(view, opts).run.line).toBe('not reported yet');
  });
});

describe('toTaskView · Handed off', () => {
  it('when, the brief and its docs; none, none', () => {
    const at = '2026-11-12T06:00:00Z';
    const v = toTaskView(F.taskView({ handoff: { at, body_md: 'Move the header onto the shell.', docs: ['coord/a.md'] } }), opts);
    expect(v.handoff).toEqual({ label: `Handed off · ${hhmm(at)}`, brief: 'Move the header onto the shell.', docs: ['coord/a.md'] });
    expect(toTaskView(F.taskView({ handoff: null }), opts).handoff).toBeNull();
  });
});
