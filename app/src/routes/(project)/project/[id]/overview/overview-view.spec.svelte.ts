import { describe, it, expect } from 'vitest';
import {
  ftrDisplay,
  projectEyebrow,
  repoChipLabel,
  heroContent,
  statBlocks,
  sessionDuration,
  relativeTime,
  sessionRows,
} from './overview-view.svelte.js';
import type {
  ProjectOverviewFolder,
  ProjectOverviewTopRec,
  ProjectOverviewStats,
  ProjectOverviewSession,
} from '$lib/types.js';

const folder = (over: Partial<ProjectOverviewFolder> = {}): ProjectOverviewFolder => ({
  id: 'f1',
  name: 'frontend',
  role: null,
  primary: true,
  ...over,
});

const rec = (over: Partial<ProjectOverviewTopRec> = {}): ProjectOverviewTopRec => ({
  id: 'r1',
  title: 'Extract the auth guard',
  why: 'Three sessions duplicated the same guard.',
  evidence: ['s-2891', 's-2889'],
  defaultAcp: 'claude-code',
  ...over,
});

const stats = (over: Partial<ProjectOverviewStats> = {}): ProjectOverviewStats => ({
  sessions7d: 28,
  sessions7dCorrected: 3,
  memories: { total: 11, readyToShare: 2, toMerge: 1 },
  docDrift: { open: 3, referencedDocs: 18 },
  ...over,
});

const session = (over: Partial<ProjectOverviewSession> = {}): ProjectOverviewSession => ({
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

  it('reads warning tone only when the project is flagged', () => {
    expect(ftrDisplay({ ftr: 0.2, warn: true }).toneClass).toBe('text-warning');
    expect(ftrDisplay({ ftr: 0.9, warn: false }).toneClass).toBe('text-ink');
  });

  it('yields a null pct (render "—") when there is no FTR data — never a fabricated 0%', () => {
    expect(ftrDisplay({ ftr: null, warn: false }).pct).toBeNull();
  });
});

describe('projectEyebrow', () => {
  it('shows the client when present', () => {
    expect(projectEyebrow({ client: 'Acme' })).toBe('Project · Acme');
  });

  it('falls back to "internal" for a null or blank client', () => {
    expect(projectEyebrow({ client: null })).toBe('Project · internal');
    expect(projectEyebrow({ client: '   ' })).toBe('Project · internal');
  });
});

describe('repoChipLabel', () => {
  it('labels the repo count only when the project spans more than one folder', () => {
    expect(repoChipLabel([folder(), folder({ id: 'f2' }), folder({ id: 'f3' })])).toBe('3 repos');
  });

  it('returns null for a single-folder (or empty) project', () => {
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
    expect(sessionRows([session({ ftr: false })], now)[0].timeToneClass).toBe('text-ink-mute');
  });

  it('passes the folder role through, including null (no chip)', () => {
    expect(sessionRows([session({ role: 'library' })], now)[0].role).toBe('library');
    expect(sessionRows([session({ role: null })], now)[0].role).toBeNull();
  });
});
