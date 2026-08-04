import { describe, it, expect } from 'vitest';
import {
  bucketProjects,
  projectStatus,
  projectIcon,
  lastSessionLabel,
  isWarning,
  type EnrichedProject,
} from './buckets.js';

const p = (over: Partial<EnrichedProject> = {}): EnrichedProject => ({
  id: 'p',
  name: 'unknown',
  maturity: 'learning',
  stack: {},
  ftr14d: 0,
  sessions7d: 0,
  warn: false,
  repos_count: 0,
  libs_count: 0,
  last_session_at: null,
  vision: null,
  ...over,
});

describe('projectStatus', () => {
  it('is active when sessions ran in the last 7 days', () => {
    expect(projectStatus(p({ sessions7d: 3 }))).toBe('active');
  });

  it('is dormant with no recent sessions and a live maturity', () => {
    expect(projectStatus(p({ sessions7d: 0, maturity: 'learning' }))).toBe('dormant');
  });

  it('is archived when maturity is archived, even with recent sessions', () => {
    expect(projectStatus(p({ sessions7d: 9, maturity: 'archived' }))).toBe('archived');
  });
});

describe('bucketProjects', () => {
  it('routes each project into exactly one bucket', () => {
    const b = bucketProjects([
      p({ id: '1', name: 'sensei', sessions7d: 5 }),
      p({ id: '2', name: 'rokkit', sessions7d: 0 }),
      p({ id: '3', name: 'old', maturity: 'archived', sessions7d: 0 }),
    ]);
    expect(b.active.map((x) => x.id)).toEqual(['1']);
    expect(b.dormant.map((x) => x.id)).toEqual(['2']);
    expect(b.archived.map((x) => x.id)).toEqual(['3']);
  });

  it('chip counts sum to all (buckets are mutually exclusive)', () => {
    const projects = [
      p({ id: '1', sessions7d: 5 }),
      p({ id: '2', sessions7d: 0 }),
      p({ id: '3', sessions7d: 2, maturity: 'archived' }),
      p({ id: '4', sessions7d: 0, maturity: 'archived' }),
    ];
    const b = bucketProjects(projects);
    expect(b.active.length + b.dormant.length + b.archived.length).toBe(projects.length);
  });

  it('an archived project with recent sessions is archived, not active', () => {
    const b = bucketProjects([p({ id: '1', sessions7d: 7, maturity: 'archived' })]);
    expect(b.archived.map((x) => x.id)).toEqual(['1']);
    expect(b.active).toEqual([]);
  });

  it('active is sorted by most-recent session first, ties by name asc', () => {
    const b = bucketProjects([
      p({ id: '1', name: 'zulu', sessions7d: 1, last_session_at: '2026-08-04T00:00:00Z' }),
      p({ id: '2', name: 'alpha', sessions7d: 1, last_session_at: '2026-08-02T00:00:00Z' }),
      p({ id: '3', name: 'mike', sessions7d: 1, last_session_at: '2026-08-02T00:00:00Z' }),
    ]);
    // zulu is newest; alpha & mike share the older timestamp → tie broken by name asc
    expect(b.active.map((x) => x.name)).toEqual(['zulu', 'alpha', 'mike']);
  });

  it('dormant and archived are sorted most-recent first; a null last-session sorts last', () => {
    const b = bucketProjects([
      p({ id: '1', name: 'zulu', last_session_at: '2026-07-01T00:00:00Z' }),
      p({ id: '2', name: 'alpha', last_session_at: null }),
      p({ id: '3', name: 'delta', maturity: 'archived', last_session_at: '2026-06-01T00:00:00Z' }),
      p({ id: '4', name: 'bravo', maturity: 'archived', last_session_at: '2026-08-01T00:00:00Z' }),
    ]);
    // dormant: zulu (has a session) before alpha (never ran → sorts last)
    expect(b.dormant.map((x) => x.name)).toEqual(['zulu', 'alpha']);
    // archived: bravo (Aug) before delta (Jun)
    expect(b.archived.map((x) => x.name)).toEqual(['bravo', 'delta']);
  });
});

describe('projectIcon', () => {
  it('uses an absolute-URL image value as-is (author/custom icon)', () => {
    expect(projectIcon(p({ icon: { kind: 'image', value: 'http://x/y.png' } }))).toEqual({
      kind: 'image',
      src: 'http://x/y.png',
      glyph: '場',
    });
  });

  it('passes a data-URI image value straight through', () => {
    const value = 'data:image/svg+xml;base64,AAA';
    expect(projectIcon(p({ icon: { kind: 'image', value } }))).toEqual({
      kind: 'image',
      src: value,
      glyph: '場',
    });
  });

  it('routes a repo-relative image value through the daemon serve route', () => {
    expect(
      projectIcon(p({ id: 'p-9', icon: { kind: 'image', value: 'assets/logo.svg' } }), 'http://127.0.0.1:7744'),
    ).toEqual({
      kind: 'image',
      src: 'http://127.0.0.1:7744/api/projects/p-9/icon',
      glyph: '場',
    });
  });

  it('serve route defaults to a bare path when no base is given', () => {
    expect(projectIcon(p({ id: 'a b', icon: { kind: 'image', value: 'logo.png' } }))).toEqual({
      kind: 'image',
      src: '/api/projects/a%20b/icon',
      glyph: '場',
    });
  });

  it('uses the kanji glyph when the icon is a kanji with a value', () => {
    expect(projectIcon(p({ icon: { kind: 'kanji', value: '禅' } }))).toEqual({
      kind: 'kanji',
      glyph: '禅',
    });
  });

  it('falls back to 場 for an empty icon object', () => {
    expect(projectIcon(p({ icon: {} }))).toEqual({ kind: 'kanji', glyph: '場' });
  });

  it('falls back to 場 for a null icon', () => {
    expect(projectIcon(p({ icon: null }))).toEqual({ kind: 'kanji', glyph: '場' });
  });

  it('falls back to 場 for an image icon with no value', () => {
    expect(projectIcon(p({ icon: { kind: 'image' } }))).toEqual({ kind: 'kanji', glyph: '場' });
  });
});

describe('lastSessionLabel', () => {
  const NOW = Date.parse('2026-07-07T12:00:00Z');
  const isoAgo = (ms: number): string => new Date(NOW - ms).toISOString();

  it('renders an em dash for null or unparseable timestamps', () => {
    expect(lastSessionLabel(null, NOW)).toBe('—');
    expect(lastSessionLabel('not-a-date', NOW)).toBe('—');
  });

  it('coarsens the span with age', () => {
    expect(lastSessionLabel(isoAgo(30_000), NOW)).toBe('just now');
    expect(lastSessionLabel(isoAgo(5 * 60_000), NOW)).toBe('5m');
    expect(lastSessionLabel(isoAgo(3 * 3_600_000), NOW)).toBe('3h');
    expect(lastSessionLabel(isoAgo(2 * 86_400_000), NOW)).toBe('2d');
    expect(lastSessionLabel(isoAgo(14 * 86_400_000), NOW)).toBe('2w');
    expect(lastSessionLabel(isoAgo(60 * 86_400_000), NOW)).toBe('2mo');
    expect(lastSessionLabel(isoAgo(400 * 86_400_000), NOW)).toBe('1y');
  });
});

describe('isWarning', () => {
  it('flags any open drift on an active project', () => {
    expect(isWarning({ open_drift_count: 1, ftr_7d: 1 }, 3)).toBe(true);
  });

  it('flags a 7-day FTR below 60% on an active project', () => {
    expect(isWarning({ open_drift_count: 0, ftr_7d: 0.59 }, 3)).toBe(true);
  });

  it('stays quiet when both signals are healthy', () => {
    expect(isWarning({ open_drift_count: 0, ftr_7d: 0.6 }, 3)).toBe(false);
    expect(isWarning({ open_drift_count: 0, ftr_7d: 1 }, 3)).toBe(false);
  });

  it('never warns a dormant project (no sessions → no FTR signal to warn about)', () => {
    // The wire's zero-FTR fallback (ftr_7d: 0) must NOT light every dormant
    // project amber — the sessions7d guard suppresses it.
    expect(isWarning({ open_drift_count: 0, ftr_7d: 0 }, 0)).toBe(false);
    expect(isWarning({ open_drift_count: 5, ftr_7d: 0 }, 0)).toBe(false);
  });
});
