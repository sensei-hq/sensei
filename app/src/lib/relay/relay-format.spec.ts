import { describe, expect, it } from 'vitest';
import { crewRepoLabel, currentStepKey } from './relay-format.js';
import type { RelayTaskStep } from '$lib/relay-api.js';

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
