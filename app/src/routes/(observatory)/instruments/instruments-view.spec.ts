import { describe, it, expect } from 'vitest';
import {
  heroCopy, initialTab, filterGroups, collapsedFor, fmtPayload, toolKindView, callVerdictView
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

  it('formats payloads (durations are lib/format fmtMs)', () => {
    expect(fmtPayload(null)).toBe('—');
    expect(fmtPayload('raw')).toBe('raw');
    expect(fmtPayload({ a: 1 })).toBe('{\n  "a": 1\n}');
  });
});

// Audit item 8 (2026-10-28): a tool's kind was coloured three times and a
// call's verdict three times, in the page's markup.
describe('toolKindView', () => {
  it('gives each kind one label, colour and hint', () => {
    expect(toolKindView('action')).toEqual({ label: 'action', tone: 'accent', text: 'text-accent', edge: 'var(--accent)', hint: 'performs an operation', runLabel: 'Run →' });
    expect(toolKindView('query')).toEqual({ label: 'query', tone: 'success', text: 'text-success', edge: 'var(--success)', hint: 'returns information', runLabel: 'Query →' });
    expect(toolKindView('other')).toEqual({ label: 'call', tone: 'muted', text: 'text-ink-soft', edge: 'var(--ink-soft)', hint: 'returns information', runLabel: 'Query →' });
  });
});

describe('callVerdictView', () => {
  it('gives each call verdict one glyph and colour', () => {
    expect(callVerdictView({ verdict: 'used', success: true })).toEqual({ glyph: '用', text: 'text-success', edge: 'var(--success)' });
    expect(callVerdictView({ verdict: 'partial', success: true })).toEqual({ glyph: '部', text: 'text-warning', edge: 'var(--warning)' });
    expect(callVerdictView({ verdict: 'ignored', success: true })).toEqual({ glyph: '無', text: 'text-ink-soft', edge: 'var(--ink-soft)' });
    expect(callVerdictView({ verdict: null, success: false })).toEqual({ glyph: '✗', text: 'text-accent', edge: 'var(--accent)' });
    expect(callVerdictView({ verdict: null, success: true })).toEqual({ glyph: '問', text: 'text-accent', edge: 'var(--accent)' });
  });
});
