import { describe, expect, it } from 'vitest';
import { crewRepoLabel } from './relay-format.js';

describe('crewRepoLabel', () => {
  it('names the crew’s repository', () => {
    expect(crewRepoLabel({ repo_key: 'github.com/sensei-hq/sensei' })).toBe('github.com/sensei-hq/sensei');
  });

  it('reads a repo-less crew as "no repository" (Q18)', () => {
    expect(crewRepoLabel({ repo_key: null })).toBe('no repository');
  });
});
