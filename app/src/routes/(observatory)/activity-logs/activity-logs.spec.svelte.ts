import { describe, it, expect } from 'vitest';
import type { LogRow } from '$lib/types.js';
import {
  DEFAULT_FILTERS,
  DEFAULT_LIMIT,
  MAX_LIMIT,
  filtersFromParams,
  buildLogsQuery,
  levelTone,
  moduleOf,
  relativeTime,
  absoluteTime,
  matchesSearch,
  searchRows,
  hasPayload,
  payloadSections,
  distinctSources,
  distinctModules,
  ActivityLogsView,
} from './activity-logs.svelte.js';

function makeRow(over: Partial<LogRow> = {}): LogRow {
  return {
    id: over.id ?? 'r1',
    level: over.level ?? 'info',
    source: over.source ?? null,
    module: 'module' in over ? over.module : 'tasks',
    logged_at: over.logged_at ?? '2026-07-13T02:19:29.612354+00:00',
    message: over.message ?? 'task_completed',
    context: 'context' in over ? (over.context ?? null) : { folder: 'Developer' },
    data: over.data ?? null,
    error: over.error ?? null,
  };
}

describe('filtersFromParams', () => {
  it('returns defaults when the URL has no params', () => {
    expect(filtersFromParams(new URLSearchParams())).toEqual(DEFAULT_FILTERS);
  });

  it('reads level / source / module / since through verbatim', () => {
    const p = new URLSearchParams('level=warn&source=daemon&module=scanner&since=1h');
    expect(filtersFromParams(p)).toEqual({
      level: 'warn',
      source: 'daemon',
      module: 'scanner',
      since: '1h',
      limit: DEFAULT_LIMIT,
    });
  });

  it('clamps limit to [1, MAX_LIMIT] and falls back for junk', () => {
    expect(filtersFromParams(new URLSearchParams('limit=50')).limit).toBe(50);
    expect(filtersFromParams(new URLSearchParams('limit=99999')).limit).toBe(MAX_LIMIT);
    expect(filtersFromParams(new URLSearchParams('limit=0')).limit).toBe(DEFAULT_LIMIT);
    expect(filtersFromParams(new URLSearchParams('limit=abc')).limit).toBe(DEFAULT_LIMIT);
  });
});

describe('buildLogsQuery', () => {
  it('is empty when every filter is at its default', () => {
    expect(buildLogsQuery(DEFAULT_FILTERS)).toBe('');
  });

  it('omits empty filters and the default since/limit', () => {
    expect(buildLogsQuery({ ...DEFAULT_FILTERS, level: 'error' })).toBe('?level=error');
  });

  it('serialises the active filters', () => {
    const qs = buildLogsQuery({ level: 'warn', source: 'daemon', module: 'tasks', since: '1h', limit: 500 });
    const parsed = new URLSearchParams(qs);
    expect(parsed.get('level')).toBe('warn');
    expect(parsed.get('source')).toBe('daemon');
    expect(parsed.get('module')).toBe('tasks');
    expect(parsed.get('since')).toBe('1h');
    expect(parsed.get('limit')).toBe('500');
  });

  it('round-trips through filtersFromParams', () => {
    const filters = { level: 'info', source: '', module: 'watchdog', since: '24h', limit: 100 };
    const qs = buildLogsQuery(filters);
    expect(filtersFromParams(new URLSearchParams(qs))).toEqual(filters);
  });
});

describe('levelTone', () => {
  it('maps each known level to a rokkit role token', () => {
    expect(levelTone('error').text).toBe('text-error');
    expect(levelTone('warn').text).toBe('text-warning');
    expect(levelTone('info').text).toBe('text-info');
    expect(levelTone('debug').text).toBe('text-ink-soft');
    expect(levelTone('trace').text).toBe('text-ink-faint');
  });

  it('falls back to the neutral ramp for an unknown level', () => {
    expect(levelTone('nope')).toEqual({ text: 'text-ink-mute', bg: 'bg-paper-mute' });
  });
});

describe('moduleOf', () => {
  it('reads the module column', () => {
    expect(moduleOf(makeRow({ module: 'scanner' }))).toBe('scanner');
  });
  it('is null when absent or empty', () => {
    expect(moduleOf(makeRow({ module: null }))).toBeNull();
    expect(moduleOf(makeRow({ module: '' }))).toBeNull();
  });
});

describe('relativeTime', () => {
  const base = Date.parse('2026-07-13T02:19:29.612354+00:00');
  it('renders human buckets', () => {
    expect(relativeTime(base, '2026-07-13T02:19:29.612354+00:00')).toBe('just now');
    expect(relativeTime(base + 30_000, '2026-07-13T02:19:29.612354+00:00')).toBe('30s ago');
    expect(relativeTime(base + 5 * 60_000, '2026-07-13T02:19:29.612354+00:00')).toBe('5m ago');
    expect(relativeTime(base + 3 * 3600_000, '2026-07-13T02:19:29.612354+00:00')).toBe('3h ago');
    expect(relativeTime(base + 2 * 86400_000, '2026-07-13T02:19:29.612354+00:00')).toBe('2d ago');
  });
  it('falls back to the raw string for an unparseable timestamp', () => {
    expect(relativeTime(base, 'not-a-date')).toBe('not-a-date');
  });
});

describe('absoluteTime', () => {
  it('formats a parseable timestamp and passes junk through', () => {
    expect(absoluteTime('2026-07-13T02:19:29Z')).not.toBe('2026-07-13T02:19:29Z');
    expect(absoluteTime('junk')).toBe('junk');
  });
});

describe('search', () => {
  it('matches over message, module and source, case-insensitively', () => {
    const row = makeRow({ message: 'task_completed', source: 'daemon', module: 'tasks' });
    expect(matchesSearch(row, 'COMPLETED')).toBe(true);
    expect(matchesSearch(row, 'daemon')).toBe(true);
    expect(matchesSearch(row, 'tasks')).toBe(true);
    expect(matchesSearch(row, 'scanner')).toBe(false);
  });

  it('an empty query matches everything and returns the same reference', () => {
    const rows = [makeRow()];
    expect(searchRows(rows, '   ')).toBe(rows);
  });

  it('filters the list on a non-empty query', () => {
    const rows = [
      makeRow({ id: 'a', message: 'scan_started' }),
      makeRow({ id: 'b', message: 'task_completed' }),
    ];
    expect(searchRows(rows, 'scan').map((r) => r.id)).toEqual(['a']);
  });
});

describe('payloadSections / hasPayload', () => {
  it('drops nulls and empty objects, orders error first', () => {
    const row = makeRow({
      context: { folder: 'Developer' },
      data: { duration_ms: 3 },
      error: { message: 'boom' },
    });
    expect(payloadSections(row).map((s) => s.label)).toEqual(['error', 'context', 'data']);
    expect(hasPayload(row)).toBe(true);
  });

  it('has no payload when context is empty and data/error are null', () => {
    const row = makeRow({ context: {}, data: null, error: null });
    expect(payloadSections(row)).toEqual([]);
    expect(hasPayload(row)).toBe(false);
  });

  it('pretty-prints the json', () => {
    const [section] = payloadSections(makeRow({ context: { folder: 'Developer' } }));
    expect(section.label).toBe('context');
    expect(section.json).toContain('\n');
    expect(section.json).toContain('"folder": "Developer"');
  });
});

describe('distinct option lists', () => {
  const rows = [
    makeRow({ source: 'daemon', module: 'tasks' }),
    makeRow({ source: null, module: 'watchdog' }),
    makeRow({ source: 'daemon', module: 'tasks' }),
  ];
  it('collects distinct non-null sources, sorted', () => {
    expect(distinctSources(rows, '')).toEqual(['daemon']);
  });
  it('unions the active selection so a narrowed page keeps it selectable', () => {
    expect(distinctSources(rows, 'cli')).toEqual(['cli', 'daemon']);
  });
  it('collects distinct modules, sorted', () => {
    expect(distinctModules(rows, '')).toEqual(['tasks', 'watchdog']);
  });
});

describe('ActivityLogsView', () => {
  it('toggles the expanded row and clears it', () => {
    const v = new ActivityLogsView();
    expect(v.expandedId).toBeNull();
    v.toggle('r1');
    expect(v.expandedId).toBe('r1');
    v.toggle('r1');
    expect(v.expandedId).toBeNull();
    v.toggle('r2');
    v.clear();
    expect(v.expandedId).toBeNull();
  });
});
