import { describe, it, expect } from 'vitest';
import {
  statusMeta,
  toneClass,
  splitSections,
  diffStats,
  type ConsolidatedRuleset,
} from './consolidation-view';

const ruleset = (over: Partial<ConsolidatedRuleset> = {}): ConsolidatedRuleset => ({
  id: 'r1',
  version: 3,
  content: '# Ruleset\n\nlead line\n\n## Style\n\nuse tokens\n\n## Testing\n\nTDD',
  conflicts: [],
  model: 'gemma4',
  status: 'proposed',
  ...over,
});

describe('statusMeta', () => {
  it('marks only a proposed version approvable', () => {
    expect(statusMeta('proposed').approvable).toBe(true);
    expect(statusMeta('approved').approvable).toBe(false);
    expect(statusMeta('superseded').approvable).toBe(false);
  });

  it('maps each status to its tone', () => {
    expect(statusMeta('proposed').tone).toBe('accent');
    expect(statusMeta('approved').tone).toBe('success');
    expect(statusMeta('superseded').tone).toBe('ink');
  });

  it('degrades an unknown/absent status to a muted label, never throwing', () => {
    expect(statusMeta(undefined).tone).toBe('ink');
    expect(statusMeta(null).label).toBe('unknown');
    expect(statusMeta('weird').label).toBe('weird');
    expect(statusMeta('weird').approvable).toBe(false);
  });
});

describe('toneClass', () => {
  it('maps tone + kind to a named-token class', () => {
    expect(toneClass('success')).toBe('text-success');
    expect(toneClass('accent', 'bg')).toBe('bg-accent');
    expect(toneClass('ink', 'border')).toBe('border-ink-mute');
  });
});

describe('splitSections', () => {
  it('splits markdown on top-level headings, keeping preamble as an unheaded section', () => {
    const secs = splitSections(ruleset().content);
    expect(secs.map((s) => s.heading)).toEqual(['Ruleset', 'Style', 'Testing']);
    expect(secs[1].body).toBe('use tokens');
  });

  it('keeps leading text before the first heading as a preamble section', () => {
    const secs = splitSections('intro paragraph\n\n## First\n\nbody');
    expect(secs[0]).toEqual({ heading: '', body: 'intro paragraph' });
    expect(secs[1].heading).toBe('First');
  });

  it('normalises CRLF and returns [] for empty content', () => {
    expect(splitSections('')).toEqual([]);
    const secs = splitSections('## A\r\nx\r\n## B\r\ny');
    expect(secs.map((s) => s.heading)).toEqual(['A', 'B']);
  });
});

describe('diffStats', () => {
  it('shows the reduction from N raw rules to one merged ruleset', () => {
    const stats = diffStats(ruleset(), 7);
    const disk = stats.find((s) => s.label === 'Rules on disk');
    expect(disk).toBeDefined();
    expect(disk!.before).toBe('7');
    expect(disk!.after).toBe('1');
    expect(disk!.delta).toBe('-6');
    expect(disk!.tone).toBe('accent');
  });

  it('omits the rules-on-disk row when the source count is unknown', () => {
    const stats = diffStats(ruleset(), null);
    expect(stats.some((s) => s.label === 'Rules on disk')).toBe(false);
  });

  it('counts headed sections and the version', () => {
    const stats = diffStats(ruleset(), null);
    expect(stats.find((s) => s.label === 'Sections')!.after).toBe('3');
    expect(stats.find((s) => s.label === 'Version')!.after).toBe('v3');
  });

  it('flags a non-empty conflicts list with the accent tone', () => {
    const withConflict = ruleset({ conflicts: [{ rule: 'x' }] });
    const conflicts = diffStats(withConflict, null).find((s) => s.label === 'Conflicts');
    expect(conflicts!.after).toBe('1');
    expect(conflicts!.tone).toBe('accent');
  });
});
