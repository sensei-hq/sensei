import { describe, it, expect } from 'vitest';
import {
  heroCopy, initialTab, filterGroups, collapsedFor, fmtPayload, fmtDuration,
} from './instruments-view';
import type { PlaygroundGroup } from './playground-groups';

const group = (id: string, tools: Array<[string, string]>): PlaygroundGroup => ({
  id, kanji: '具', name: id, installed: true, primary: id === 'sensei', serverId: id,
  tools: tools.map(([name, summary]) => ({ id: `${id}.${name}`, serverId: id, name, kind: 'query', summary, schema: undefined })),
});

describe('instruments view', () => {
  it('opens the tab a deep link names, else the playground', () => {
    expect(initialTab('replay')).toBe('replay');
    expect(initialTab('insights')).toBe('insights');
    expect(initialTab('bogus')).toBe('playground');
    expect(initialTab(null)).toBe('playground');
  });

  it('gives each tab its own hero copy', () => {
    expect(heroCopy('replay').tagline).toBe('Every instrument call, in order.');
    expect(heroCopy('insights').tagline).toBe('Toolset health.');
    expect(heroCopy('playground').tagline).toBe('Sensei · what your instruments can do.');
  });

  it('filters tools by name or summary and drops groups with no hits', () => {
    const groups = [group('sensei', [['get_callers', 'who calls'], ['search', 'find symbols']]), group('gh', [['issues', 'list']])];
    expect(filterGroups(groups, '')).toBe(groups);
    const hits = filterGroups(groups, 'SYMBOL');
    expect(hits.map((g) => g.id)).toEqual(['sensei']);
    expect(hits[0].tools.map((t) => t.name)).toEqual(['search']);
  });

  it('expands every group with a hit while searching, else keeps the user’s choice', () => {
    const groups = [group('sensei', [['a', '']]), group('gh', [['b', '']])];
    const chosen = { sensei: false, gh: true };
    expect(collapsedFor(groups, chosen, '')).toBe(chosen);
    expect(collapsedFor(groups, chosen, 'b')).toEqual({ sensei: false, gh: false });
  });

  it('formats payloads and durations', () => {
    expect(fmtPayload(null)).toBe('—');
    expect(fmtPayload('raw')).toBe('raw');
    expect(fmtPayload({ a: 1 })).toBe('{\n  "a": 1\n}');
    expect(fmtDuration(null)).toBe('—');
    expect(fmtDuration(250)).toBe('250 ms');
    expect(fmtDuration(1500)).toBe('1.50 s');
  });
});
