import { describe, expect, it } from 'vitest';
import { crewRepoLabel, crewSizeContext, currentStepKey, taskStateLabel } from './format.js';
import type { ApiError } from '../result.js';
import type { RelayLane, RelayTaskStep } from './types.js';
import * as F from './fixtures.js';

describe('crewSizeContext', () => {
  // #394: the main window's context line reads the crew's size — the assistant
  // sessions the read shows — never the crew count, which is always one.
  const read = (...lanes: RelayLane[]) => ({ lanes });

  it('counts the assistant sessions the read shows, across every crew it shows', () => {
    const lanes = read(
      F.coordinatorLane(),
      F.lane(),
      F.lane({ id: F.LANE.L9, crew_id: F.LOOSE_CREW_ID, code: 'L9' }),
    );
    expect(crewSizeContext(lanes, null)).toBe('3 assistants');
  });

  it('one assistant reads singular', () => {
    expect(crewSizeContext(read(F.coordinatorLane()), null)).toBe('1 assistant');
  });

  it('no crew yet is an honest zero: the read answered and showed nobody', () => {
    expect(crewSizeContext(read(), null)).toBe('0 assistants');
  });

  it('a read that failed says so, never "0 assistants" (D83)', () => {
    const error: ApiError = { status: 503, message: 'Service Unavailable' };
    expect(crewSizeContext(null, null)).toBe('assistants not reported');
    // A re-read that failed keeps the stale crew in hand; the line still does not count it.
    expect(crewSizeContext(read(F.coordinatorLane()), error)).toBe('assistants not reported');
  });
});

describe('crewRepoLabel', () => {
  it('names the crew’s repository', () => {
    expect(crewRepoLabel({ id: '019a5e2c-4b1d-7c00-8d2e-00000000c7f3', repo_key: 'github.com/sensei-hq/sensei' })).toBe(
      'github.com/sensei-hq/sensei'
    );
  });

  it('reads a repo-less crew as "no repository · crew XXXX" (Q18 L1)', () => {
    expect(crewRepoLabel({ id: '019a5e2c-4b1d-7c00-8d2e-00000000c7f3', repo_key: null })).toBe(
      'no repository · crew c7f3'
    );
  });

  it('two repo-less crews read distinct (Q18 L1)', () => {
    const a = crewRepoLabel({ id: '019a5e2c-4b1d-7c00-8d2e-00000000c7f3', repo_key: null });
    const b = crewRepoLabel({ id: '019a5e2c-4b1d-7c00-8d2e-00000000b10c', repo_key: null });
    expect(a).not.toBe(b);
  });
});

describe('taskStateLabel', () => {
  it('a ready task reads as in the pool (queue cutover)', () => {
    expect(taskStateLabel('ready')).toBe('in the pool');
    expect(taskStateLabel('in_review')).toBe('in review');
  });
});

describe('currentStepKey', () => {
  // F8: the checklist highlights the first open step as current.
  const step = (key: string, state: RelayTaskStep['state']): RelayTaskStep => ({
    key,
    title: key,
    state,
    reason: null,
    updated_at: '2026-11-06T10:11:00Z',
  });

  it('names the first open step, skipping done and skipped', () => {
    expect(
      currentStepKey([step('a', 'done'), step('b', 'skipped'), step('c', 'open'), step('d', 'open')]),
    ).toBe('c');
  });

  it('reads null when nothing is open', () => {
    expect(currentStepKey([step('a', 'done'), step('b', 'skipped')])).toBeNull();
    expect(currentStepKey([])).toBeNull();
  });
});
