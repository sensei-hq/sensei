import { describe, it, expect } from 'vitest';
import { recentProjects, chipProjects, matchProjects } from './project-filter.js';
import type { InsightProjectRef } from '$lib/types.js';

const p = (id: string, name: string, last: string | null): InsightProjectRef => ({
  id, name, kanji: '場', last_session_at: last,
});

const PROJECTS: InsightProjectRef[] = [
  p('a', 'alpha', '2026-08-01T00:00:00Z'),
  p('b', 'bravo', '2026-08-04T00:00:00Z'), // most recent
  p('c', 'charlie', null),                  // never ran → last
  p('d', 'delta', '2026-08-03T00:00:00Z'),
  p('e', 'echo', '2026-08-02T00:00:00Z'),
];

describe('recentProjects', () => {
  it('returns the N most-recent by last_session_at desc', () => {
    expect(recentProjects(PROJECTS, 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });

  it('sorts a never-run project last and breaks ties by name', () => {
    const tied = [
      p('x', 'zulu', '2026-08-02T00:00:00Z'),
      p('y', 'mike', '2026-08-02T00:00:00Z'),
      p('z', 'never', null),
    ];
    expect(recentProjects(tied, 3).map((x) => x.name)).toEqual(['mike', 'zulu', 'never']);
  });

  it('does not mutate the input array', () => {
    const copy = [...PROJECTS];
    recentProjects(PROJECTS, 3);
    expect(PROJECTS).toEqual(copy);
  });
});

describe('chipProjects', () => {
  it('is just the recent N when nothing is selected', () => {
    expect(chipProjects(PROJECTS, null, 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });

  it('does not duplicate a selected project already in the recent set', () => {
    expect(chipProjects(PROJECTS, 'b', 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });

  it('appends the selected project when it is NOT in the recent set', () => {
    // 'a' (alpha) and 'c' (charlie) are outside the top-3 → shown as an extra chip.
    expect(chipProjects(PROJECTS, 'a', 3).map((x) => x.id)).toEqual(['b', 'd', 'e', 'a']);
    expect(chipProjects(PROJECTS, 'c', 3).map((x) => x.id)).toEqual(['b', 'd', 'e', 'c']);
  });

  it('ignores an unknown selected id', () => {
    expect(chipProjects(PROJECTS, 'nope', 3).map((x) => x.id)).toEqual(['b', 'd', 'e']);
  });
});

describe('matchProjects', () => {
  it('is empty for a blank query (dropdown only shows while typing)', () => {
    expect(matchProjects(PROJECTS, '   ')).toEqual([]);
  });

  it('matches case-insensitively on name substring', () => {
    expect(matchProjects(PROJECTS, 'AL').map((x) => x.id)).toEqual(['a']); // alpha
    expect(matchProjects(PROJECTS, 'o').map((x) => x.id).sort()).toEqual(['b', 'e']); // bravo, echo
  });

  it('caps the number of matches', () => {
    const many = Array.from({ length: 20 }, (_, i) => p(`m${i}`, `match-${i}`, null));
    expect(matchProjects(many, 'match', 8)).toHaveLength(8);
  });
});
