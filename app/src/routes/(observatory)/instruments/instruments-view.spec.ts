import { describe, it, expect } from 'vitest';
import {
  heroCopy, initialTab, pickToolDefaults, filterGroups, collapsedFor, fmtPayload, fmtDuration,
} from './instruments-view';
import type { McpToolManifest } from '$lib/types.js';
import type { PlaygroundGroup } from './playground-groups';

const manifest = (inputs: McpToolManifest['inputs']): McpToolManifest => ({
  mcp: 'sensei', id: 'sensei.get_callers', name: 'get_callers', kind: 'query', summary: '', inputs,
  example: {} as McpToolManifest['example'],
});

const group = (id: string, tools: Array<[string, string]>): PlaygroundGroup => ({
  id, kanji: '具', name: id, installed: true, source: id === 'sensei' ? 'sensei' : 'discovered',
  tools: tools.map(([name, summary]) => ({ id: `${id}.${name}`, mcp: id, name, kind: 'query', summary })),
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

  it('prefills repoId with the active workspace, then defaults, then placeholders', () => {
    const m = manifest([
      { key: 'repoId', kind: 'text', required: true, label: 'Repo', placeholder: 'sensei' },
      { key: 'limit', kind: 'text', required: false, label: 'Limit', default: '10' },
      { key: 'name', kind: 'text', required: true, label: 'Name', placeholder: 'process_event' },
      { key: 'free', kind: 'text', required: false, label: 'Free' },
    ]);
    expect(pickToolDefaults(m, 'atlas')).toEqual({ repoId: 'atlas', limit: '10', name: 'process_event' });
    expect(pickToolDefaults(m, null).repoId).toBe('sensei');
    expect(pickToolDefaults(null, 'atlas')).toEqual({});
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
