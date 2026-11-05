import { describe, it, expect, vi } from 'vitest';
import type { LogRow, ScheduledTask } from '$lib/types.js';
import {
  DEFAULT_FILTERS,
  DEFAULT_LIMIT,
  MAX_LIMIT,
  filtersFromParams,
  logsPageData,
  readLogs,
  LogsScreen,
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
    module: 'module' in over ? (over.module ?? null) : 'tasks',
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
    expect(levelTone('error').text).toBe('text-on-error-soft');
    expect(levelTone('warn').text).toBe('text-on-warning-soft');
    expect(levelTone('info').text).toBe('text-on-info-soft');
    expect(levelTone('debug').text).toBe('text-ink-mute');
    expect(levelTone('trace').text).toBe('text-ink-faint');
  });

  it('falls back to the neutral ramp for an unknown level', () => {
    expect(levelTone('nope')).toEqual({ text: 'text-ink-soft', bg: 'bg-paper-mute' });
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

// #96 and the no-fabrication rule: the loader fell back to [] on any failure,
// so a daemon that could not answer read as "no log entries" and "no background
// tasks". Each half now carries its own error.
describe('logsPageData', () => {
  const filters = filtersFromParams(new URLSearchParams());
  const ok = <T>(data: T) => ({ ok: true as const, data });
  const fail = { ok: false as const, error: { status: 500, message: 'boom' } };

  it('passes real data through, error-free', () => {
    const d = logsPageData(ok([]), ok({ tasks: [] }), filters);
    expect(d).toMatchObject({ rows: [], rowsError: null, tasks: [], tasksError: null });
  });

  it('says the log rows could not be read, rather than that there are none', () => {
    const d = logsPageData(fail, ok({ tasks: [] }), filters);
    expect(d.rowsError).toContain('boom');
    expect(d.tasksError).toBeNull();
  });

  it('says each failed read through the shared Notice: what failed, then why', () => {
    const d = logsPageData(fail, fail, filters);
    expect(d.rowsNotice).toEqual({ tone: 'fault', title: "Couldn't read the log", detail: 'boom', remedy: 'Retry. If it keeps failing, reload the page.' });
    expect(d.tasksNotice).toEqual({ tone: 'fault', title: "Couldn't read the background tasks", detail: 'boom', remedy: 'Retry. If it keeps failing, reload the page.' });
    expect(logsPageData(ok([]), ok({ tasks: [] }), filters)).toMatchObject({ rowsNotice: null, tasksNotice: null });
  });

  it('says the background tasks could not be read, rather than that there are none', () => {
    const d = logsPageData(ok([]), fail, filters);
    expect(d.tasksError).toContain('boom');
    expect(d.rowsError).toBeNull();
  });
});

// r2 (L3): `readLogs` is the one filters→query mapping — the loader and the
// notice's Retry both go through it — so the whole query is pinned, not just
// the two fields the page tests happen to exercise.
describe('readLogs', () => {
  const spy = () => vi.fn().mockResolvedValue({ ok: true as const, data: [] });

  it('asks for every filter the URL carries, verbatim', async () => {
    const tryGetLogs = spy();
    await readLogs({ tryGetLogs }, filtersFromParams(
      new URLSearchParams('level=warn&source=daemon&module=scanner&since=1h&limit=50'),
    ));
    expect(tryGetLogs.mock.calls[0][0]).toEqual({
      level: 'warn',
      source: 'daemon',
      module: 'scanner',
      since: '1h',
      limit: 50,
    });
  });

  it('sends no filter at all for an empty level / source / module', async () => {
    const tryGetLogs = spy();
    await readLogs({ tryGetLogs }, filtersFromParams(new URLSearchParams()));
    expect(tryGetLogs.mock.calls[0][0]).toEqual({
      level: undefined,
      source: undefined,
      module: undefined,
      since: DEFAULT_FILTERS.since,
      limit: DEFAULT_FILTERS.limit,
    });
  });
});

// r2 (N1, L0): the Retry state lives in the screen's class, as settings and
// extensions keep theirs — so the two halves, the guards and the order of the
// two writers are testable without mounting the page.
describe('LogsScreen', () => {
  const filters = DEFAULT_FILTERS;
  const ok = <T>(data: T) => ({ ok: true as const, data });
  const fail = { ok: false as const, error: { status: 502, message: 'bad gateway' } };
  const task = (name: string): ScheduledTask => ({
    name,
    description: 'scans the workspace',
    enabled: true,
    interval_secs: 300,
    window: null,
    days: null,
    last_run_at: null,
    last_ok: true,
    last_error: null,
    next_run_at: null,
    avg_ms: null,
    running_since: null,
  });
  const rows = [makeRow({ id: 'a', message: 'first_read' })];
  const screenWith = (
    rowsResult: Parameters<typeof logsPageData>[0],
    tasksResult: Parameters<typeof logsPageData>[1],
  ) => new LogsScreen(logsPageData(rowsResult, tasksResult, filters));

  // L2: the halves are independent — a log Retry never takes the workers away,
  // whatever its own answer.
  it('a log retry rewrites the log half only', async () => {
    const good = screenWith(fail, ok({ tasks: [task('scanner')] }));
    await good.retryLogs({ tryGetLogs: vi.fn().mockResolvedValue(ok(rows)) });
    expect(good.shown.rows.map((r) => r.id)).toEqual(['a']);
    expect(good.shown.rowsNotice).toBeNull();
    expect(good.shown.tasks.map((t) => t.name)).toEqual(['scanner']);

    const bad = screenWith(fail, ok({ tasks: [task('scanner')] }));
    await bad.retryLogs({ tryGetLogs: vi.fn().mockResolvedValue(fail) });
    expect(bad.shown.rowsNotice?.title).toBe("Couldn't read the log");
    expect(bad.shown.tasks.map((t) => t.name)).toEqual(['scanner']);
  });

  it('a tasks retry rewrites the tasks half only', async () => {
    const s = screenWith(ok(rows), fail);
    await s.retryTasks({
      tryGetScheduledTasks: vi.fn().mockResolvedValue(ok({ tasks: [task('scanner')] })),
    });
    expect(s.shown.tasks.map((t) => t.name)).toEqual(['scanner']);
    expect(s.shown.tasksNotice).toBeNull();
    expect(s.shown.rows.map((r) => r.id)).toEqual(['a']);
  });

  // R6/N1 at the class: the guard is what makes a double press one read, since
  // both presses land before anything re-renders.
  it('two presses of either retry are one read', async () => {
    let release: (v: unknown) => void = () => {};
    const pending = new Promise((resolve) => {
      release = resolve;
    });

    const logs = screenWith(fail, ok({ tasks: [] }));
    const tryGetLogs = vi.fn().mockReturnValue(pending);
    const a = logs.retryLogs({ tryGetLogs });
    const b = logs.retryLogs({ tryGetLogs });
    release(ok(rows));
    await Promise.all([a, b]);
    expect(tryGetLogs).toHaveBeenCalledTimes(1);

    const tasks = screenWith(ok(rows), fail);
    const tryGetScheduledTasks = vi.fn().mockReturnValue(pending);
    const c = tasks.retryTasks({ tryGetScheduledTasks });
    const d = tasks.retryTasks({ tryGetScheduledTasks });
    await Promise.all([c, d]);
    expect(tryGetScheduledTasks).toHaveBeenCalledTimes(1);
  });

  // M2 at the class: the loader's answer is the last word — the filters live in
  // the URL, so a navigation replaces a Retry's answer rather than the reverse.
  it('the loader’s answer replaces what a retry read', async () => {
    const s = screenWith(fail, ok({ tasks: [] }));
    await s.retryLogs({ tryGetLogs: vi.fn().mockResolvedValue(ok(rows)) });
    expect(s.shown.rows.map((r) => r.id)).toEqual(['a']);

    s.setData(
      logsPageData(
        ok([makeRow({ id: 'n', message: 'from_the_url' })]),
        ok({ tasks: [task('scanner')] }),
        { ...filters, level: 'error' },
      ),
    );
    expect(s.shown.rows.map((r) => r.id)).toEqual(['n']);
    expect(s.shown.filters.level).toBe('error');
    expect(s.shown.tasks.map((t) => t.name)).toEqual(['scanner']);
  });
});
