import { describe, it, expect } from 'vitest';
import {
  ftrDisplay,
  workspaceEyebrow,
  repoChipLabel,
  heroContent,
  statBlocks,
  sessionDuration,
  relativeTime,
  sessionRows,
  workspaceVisibilityLabel,
  hotspotRows,
  patternSnapshot,
  overviewPageData,
} from './overview-view.svelte.js';
import type {
  WorkspaceOverviewFolder,
  WorkspaceOverviewTopRec,
  WorkspaceOverviewStats,
  WorkspaceOverviewSession,
} from '$lib/types.js';

const folder = (over: Partial<WorkspaceOverviewFolder> = {}): WorkspaceOverviewFolder => ({
  id: 'f1',
  name: 'frontend',
  role: null,
  primary: true,
  ...over,
});

const rec = (over: Partial<WorkspaceOverviewTopRec> = {}): WorkspaceOverviewTopRec => ({
  id: 'r1',
  title: 'Extract the auth guard',
  why: 'Three sessions duplicated the same guard.',
  evidence: ['s-2891', 's-2889'],
  defaultAcp: 'claude-code',
  ...over,
});

const stats = (over: Partial<WorkspaceOverviewStats> = {}): WorkspaceOverviewStats => ({
  sessions7d: 28,
  sessions7dCorrected: 3,
  memories: { total: 11, readyToShare: 2, toMerge: 1 },
  docDrift: { open: 3, referencedDocs: 18 },
  ...over,
});

const session = (over: Partial<WorkspaceOverviewSession> = {}): WorkspaceOverviewSession => ({
  id: 'c6dbe60f-00ba-4531-bec2-778f2eaf7c43',
  title: 'Wire the overview endpoint',
  startedAt: '2026-07-08T01:00:00Z',
  completedAt: '2026-07-08T01:42:00Z',
  corrections: 0,
  ftr: true,
  role: null,
  ...over,
});

describe('ftrDisplay', () => {
  it('rounds ftr to a whole-number percentage', () => {
    expect(ftrDisplay({ ftr: 0.784, warn: false }).pct).toBe(78);
    expect(ftrDisplay({ ftr: 0.2, warn: false }).pct).toBe(20);
  });

  it('reads warning tone only when the workspace is flagged', () => {
    expect(ftrDisplay({ ftr: 0.2, warn: true }).toneClass).toBe('text-warning');
    expect(ftrDisplay({ ftr: 0.9, warn: false }).toneClass).toBe('text-ink');
  });

  it('yields a null pct (render "—") when there is no FTR data — never a fabricated 0%', () => {
    expect(ftrDisplay({ ftr: null, warn: false }).pct).toBeNull();
  });
});

describe('workspaceEyebrow', () => {
  it('shows the client when present', () => {
    expect(workspaceEyebrow({ client: 'Acme' })).toBe('Workspace · Acme');
  });

  it('falls back to "internal" for a null or blank client', () => {
    expect(workspaceEyebrow({ client: null })).toBe('Workspace · internal');
    expect(workspaceEyebrow({ client: '   ' })).toBe('Workspace · internal');
  });
});

describe('workspaceVisibilityLabel', () => {
  // The sidebar chip and the overview eyebrow share this fallback. The sidebar
  // used to render no chip at all for a client-less workspace, which read as
  // missing data rather than "this one is internal".
  it('returns the client name when there is one', () => {
    expect(workspaceVisibilityLabel('Acme')).toBe('Acme');
  });

  it.each([[null], [undefined], [''], ['   ']] as const)(
    'falls back to "internal" for %p',
    (client) => {
      expect(workspaceVisibilityLabel(client)).toBe('internal');
    },
  );

  it('is the same fallback the eyebrow uses', () => {
    expect(workspaceEyebrow({ client: null })).toContain(workspaceVisibilityLabel(null));
  });
});

describe('repoChipLabel', () => {
  it('labels the repo count only when the workspace spans more than one folder', () => {
    expect(repoChipLabel([folder(), folder({ id: 'f2' }), folder({ id: 'f3' })])).toBe('3 repos');
  });

  it('returns null for a single-folder (or empty) workspace', () => {
    expect(repoChipLabel([folder()])).toBeNull();
    expect(repoChipLabel([])).toBeNull();
  });
});

describe('heroContent', () => {
  it('renders the all-quiet state when there is no recommendation', () => {
    const h = heroContent(null);
    expect(h.quiet).toBe(true);
    expect(h.kanji).toBe('静');
    expect(h.headline).toBe('All quiet — no urgent recommendations.');
    expect(h.body).toBe('Sensei is observing. The next correction or pattern will surface here.');
    expect(h.action).toBeNull();
    expect(h.meta).toBeNull();
  });

  it('renders the top-rec headline, body, action, and evidence meta', () => {
    const h = heroContent(rec());
    expect(h.quiet).toBe(false);
    expect(h.kanji).toBe('聴');
    expect(h.headline).toBe('Extract the auth guard');
    expect(h.body).toBe('Three sessions duplicated the same guard.');
    expect(h.action).toBe('send to claude-code');
    expect(h.meta).toBe('s-2891 · s-2889');
  });

  it('omits the send action when defaultAcp is null', () => {
    expect(heroContent(rec({ defaultAcp: null })).action).toBeNull();
  });

  it('omits the meta when there is no evidence', () => {
    expect(heroContent(rec({ evidence: [] })).meta).toBeNull();
  });

  it('falls back to a neutral headline when a rec carries a blank title', () => {
    expect(heroContent(rec({ title: '' })).headline).toBe('A recommendation is ready.');
  });
});

describe('statBlocks', () => {
  it('carries each headline number and the counts backing it', () => {
    const [sessionsBlock, memoriesBlock, driftBlock] = statBlocks(stats());
    expect(sessionsBlock).toMatchObject({ value: 28, sub: '3 corrected' });
    expect(memoriesBlock).toMatchObject({ value: 11, sub: '2 to share · 1 to merge' });
    expect(driftBlock).toMatchObject({ value: 3, sub: 'of 18 referenced docs' });
  });

  it('warns on doc drift only when the open count is above zero', () => {
    expect(statBlocks(stats({ docDrift: { open: 3, referencedDocs: 18 } }))[2].toneClass).toBe('text-warning');
    expect(statBlocks(stats({ docDrift: { open: 0, referencedDocs: 18 } }))[2].toneClass).toBe('text-ink');
  });
});

describe('sessionDuration', () => {
  it('formats sub-hour, hour, and multi-day spans', () => {
    expect(sessionDuration('2026-07-08T01:00:00Z', '2026-07-08T01:42:00Z')).toBe('42m');
    expect(sessionDuration('2026-07-08T01:00:00Z', '2026-07-08T02:30:00Z')).toBe('1h 30m');
    expect(sessionDuration('2026-07-08T01:00:00Z', '2026-07-08T03:00:00Z')).toBe('2h');
    // 2 days + 3 hours
    expect(sessionDuration('2026-07-01T00:00:00Z', '2026-07-03T03:00:00Z')).toBe('2d 3h');
  });

  it('returns null when incomplete or the timestamps do not advance', () => {
    expect(sessionDuration('2026-07-08T01:00:00Z', null)).toBeNull();
    expect(sessionDuration('2026-07-08T02:00:00Z', '2026-07-08T01:00:00Z')).toBeNull();
  });
});

describe('relativeTime', () => {
  const now = Date.parse('2026-07-08T12:00:00Z');

  it('produces terse spans relative to the injected now', () => {
    expect(relativeTime('2026-07-08T11:30:00Z', now)).toBe('30m');
    expect(relativeTime('2026-07-08T09:00:00Z', now)).toBe('3h');
    expect(relativeTime('2026-07-05T12:00:00Z', now)).toBe('3d');
  });

  it('returns an empty string for a missing timestamp', () => {
    expect(relativeTime(null, now)).toBe('');
  });
});

describe('sessionRows', () => {
  const now = Date.parse('2026-07-08T02:00:00Z');

  it('builds the mono meta from a short id, duration, and correction phrase', () => {
    const [row] = sessionRows([session({ corrections: 2 })], now);
    expect(row.meta).toBe('c6dbe60f · 42m · 2 corrections');
  });

  it('says "first-try right" for a clean session', () => {
    const [row] = sessionRows([session({ corrections: 0 })], now);
    expect(row.meta).toContain('first-try right');
  });

  it('falls back to a generic title when the session title is blank', () => {
    expect(sessionRows([session({ title: '' })], now)[0].title).toBe('session');
  });

  it('tones the time column by FTR', () => {
    expect(sessionRows([session({ ftr: true })], now)[0].timeToneClass).toBe('text-success');
    expect(sessionRows([session({ ftr: false })], now)[0].timeToneClass).toBe('text-ink-soft');
  });

  it('passes the folder role through, including null (no chip)', () => {
    expect(sessionRows([session({ role: 'library' })], now)[0].role).toBe('library');
    expect(sessionRows([session({ role: null })], now)[0].role).toBeNull();
  });
});

// #85: the overview shows where edits concentrate, and which patterns are in
// use — both served by the daemon, neither shown until now.
describe('hotspotRows', () => {
  const h = (file_path: string, edit_count: number, correction_count = 0, folder = 'sensei') =>
    ({ folder, file_path, edit_count, correction_count });

  it('lists the most-edited files first, path relative to their repository', () => {
    const rows = hotspotRows([
      h('/Users/x/dev/sensei/docs/CHECKPOINT.md', 6, 5),
      h('/Users/x/dev/sensei/app/src/lib/api.ts', 9, 1),
    ]);
    expect(rows).toEqual([
      { path: 'app/src/lib/api.ts', repo: 'sensei', edits: 9, corrections: 1 },
      { path: 'docs/CHECKPOINT.md', repo: 'sensei', edits: 6, corrections: 5 },
    ]);
  });

  it('keeps the top five, and the full path when the repository is not in it', () => {
    const many = Array.from({ length: 8 }, (_, i) => h(`/tmp/f${i}.ts`, i));
    const rows = hotspotRows(many);
    expect(rows).toHaveLength(5);
    expect(rows[0]).toEqual({ path: '/tmp/f7.ts', repo: 'sensei', edits: 7, corrections: 0 });
  });
});

describe('patternSnapshot', () => {
  const pat = (name: string, instanceCount?: number) => ({ name, instanceCount });
  it('counts followed and anti-patterns and names the three most-used of each', () => {
    const snap = patternSnapshot({
      followed: [pat('a', 1), pat('b', 9), pat('c', 4), pat('d', 6)],
      antiPatterns: [pat('x'), pat('y', 2)],
    });
    expect(snap).toEqual({
      followed: 4,
      anti: 2,
      topFollowed: ['b', 'd', 'c'],
      topAnti: ['y', 'x'],
    });
  });
});

describe('overviewPageData', () => {
  const ok = <T,>(data: T) => ({ ok: true as const, data });
  const err = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

  it('carries each section, or why it could not be read — never a made-up default', () => {
    const d = overviewPageData(err, ok({ hotspots: [] }), err);
    expect(d.overview).toBeNull();
    expect(d.overviewError).toBe('Service Unavailable');
    expect(d.hotspots).toEqual([]);
    expect(d.hotspotsError).toBeNull();
    expect(d.patterns).toBeNull();
    expect(d.patternsError).toBe('Service Unavailable');
  });
});
