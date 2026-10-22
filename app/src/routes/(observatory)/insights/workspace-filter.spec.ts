import { describe, it, expect } from 'vitest';
import { recentWorkspaces, chipWorkspaces, matchWorkspaces } from './workspace-filter.js';
import type { InsightWorkspaceRef } from '$lib/types.js';

const p = (id: string, name: string, last: string | null): InsightWorkspaceRef => ({
  id, name, kanji: '場', last_session_at: last,
});

const WORKSPACES: InsightWorkspaceRef[] = [
  p('a', 'alpha', '2026-08-01T00:00:00Z'),
  p('b', 'bravo', '2026-08-04T00:00:00Z'), // most recent
  p('c', 'charlie', null),                  // never ran → last
  p('d', 'delta', '2026-08-03T00:00:00Z'),
  p('e', 'echo', '2026-08-02T00:00:00Z'),
];

describe('recentWorkspaces', () => {
  it('returns the N most-recent by last_session_at desc', () => {
    expect(recentWorkspaces(WORKSPACES, 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });

  it('sorts a never-run workspace last and breaks ties by name', () => {
    const tied = [
      p('x', 'zulu', '2026-08-02T00:00:00Z'),
      p('y', 'mike', '2026-08-02T00:00:00Z'),
      p('z', 'never', null),
    ];
    expect(recentWorkspaces(tied, 3).map((x) => x.name)).toEqual(['mike', 'zulu', 'never']);
  });

  it('does not mutate the input array', () => {
    const copy = [...WORKSPACES];
    recentWorkspaces(WORKSPACES, 3);
    expect(WORKSPACES).toEqual(copy);
  });
});

describe('chipWorkspaces', () => {
  it('is just the recent N when nothing is selected', () => {
    expect(chipWorkspaces(WORKSPACES, null, 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });

  it('does not duplicate a selected workspace already in the recent set', () => {
    expect(chipWorkspaces(WORKSPACES, 'b', 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });

  it('appends the selected workspace when it is NOT in the recent set', () => {
    // 'a' (alpha) and 'c' (charlie) are outside the top-3 → shown as an extra chip.
    expect(chipWorkspaces(WORKSPACES, 'a', 3).map((x) => x.id)).toEqual(['b', 'd', 'e', 'a']);
    expect(chipWorkspaces(WORKSPACES, 'c', 3).map((x) => x.id)).toEqual(['b', 'd', 'e', 'c']);
  });

  it('ignores an unknown selected id', () => {
    expect(chipWorkspaces(WORKSPACES, 'nope', 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });
});

describe('matchWorkspaces', () => {
  it('is empty for a blank query (dropdown only shows while typing)', () => {
    expect(matchWorkspaces(WORKSPACES, '   ')).toEqual([]);
  });

  it('matches case-insensitively on name substring', () => {
    expect(matchWorkspaces(WORKSPACES, 'AL').map((x) => x.id)).toEqual(['a']); // alpha
    expect(matchWorkspaces(WORKSPACES, 'o').map((x) => x.id).sort()).toEqual(['b', 'e']); // bravo, echo
  });

  it('caps the number of matches', () => {
    const many = Array.from({ length: 20 }, (_, i) => p(`m${i}`, `match-${i}`, null));
    expect(matchWorkspaces(many, 'match', 8)).toHaveLength(8);
  });
});
