// @vitest-environment jsdom
// Observatory · Logs — the screen as the daemon's reader sees it.
//
// The read is the loader's (+page.ts): the component draws `data` and makes no
// read of its own. The search narrows what is loaded; the level / source /
// module / since / limit controls are server-side, so they navigate with a new
// query rather than filtering in place. A failed read is the error, never "no
// logs for these filters".
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import { DEFAULT_FILTERS } from './activity-logs.svelte.js';
import type { LogRow, ScheduledTask } from '$lib/types.js';

vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/activity-logs') } }));

// D87 (tests r2, L1): the DOM hooks alone can't tell rokkit's shell from a
// hand-rolled header carrying the same attributes, so the shell itself is spied
// on and asked for this view's id. The DOM assertions stay: they pin where the
// rows and the workers sit, which the spy can't.
vi.mock('@rokkit/app', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, ScreenShell: vi.fn(m.ScreenShell as never) };
});

import { goto } from '$app/navigation';
import { ScreenShell } from '@rokkit/app';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.unstubAllGlobals();
  vi.mocked(goto).mockClear();
  vi.mocked(ScreenShell).mockClear();
});

function logRow(over: Partial<LogRow> = {}): LogRow {
  return {
    id: over.id ?? 'r1',
    level: over.level ?? 'info',
    source: 'source' in over ? (over.source ?? null) : 'daemon',
    module: 'module' in over ? (over.module ?? null) : 'tasks',
    logged_at: over.logged_at ?? new Date().toISOString(),
    message: over.message ?? 'task_completed',
    context: 'context' in over ? (over.context ?? null) : { folder: 'Developer' },
    data: over.data ?? null,
    error: over.error ?? null,
  };
}

function task(over: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    name: over.name ?? 'scanner',
    description: over.description ?? 'scans the workspace',
    enabled: over.enabled ?? true,
    interval_secs: over.interval_secs ?? 300,
    window: over.window ?? null,
    days: over.days ?? null,
    last_run_at: over.last_run_at ?? new Date(Date.now() - 120_000).toISOString(),
    last_ok: 'last_ok' in over ? (over.last_ok ?? null) : true,
    last_error: over.last_error ?? null,
    next_run_at: over.next_run_at ?? new Date(Date.now() + 180_000).toISOString(),
    avg_ms: over.avg_ms ?? 42,
    running_since: over.running_since ?? null,
  };
}

type Data = {
  rows: LogRow[];
  rowsNotice: unknown;
  rowsError: string | null;
  filters: typeof DEFAULT_FILTERS;
  capped: boolean;
  tasks: ScheduledTask[];
  tasksNotice: unknown;
  tasksError: string | null;
};

const notice = (title: string, detail: string) => ({
  tone: 'fault',
  title,
  detail,
  remedy: 'Retry. If it keeps failing, reload the page.',
});

function pageData(over: Partial<Data> = {}): Data {
  return {
    rows: over.rows ?? [],
    rowsNotice: over.rowsNotice ?? null,
    rowsError: over.rowsError ?? null,
    filters: over.filters ?? { ...DEFAULT_FILTERS },
    capped: over.capped ?? false,
    tasks: over.tasks ?? [],
    tasksNotice: over.tasksNotice ?? null,
    tasksError: over.tasksError ?? null,
  };
}

async function mount(over: Partial<Data> = {}) {
  const m = mountComponent(Page, { data: pageData(over) } as never);
  cleanup.push(m.destroy);
  await tick();
  return m.container;
}

const rowHeaders = (c: HTMLElement) => c.querySelectorAll('[data-log-row-header]');

describe('Observatory · Logs — the loader’s read', () => {
  it('draws the rows it was given, with no read of its own', async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [logRow({ id: 'a', message: 'scan_started' }), logRow({ id: 'b', message: 'task_completed' })],
    });
    await vi.waitFor(() => expect(rowHeaders(c).length).toBe(2));
    expect(c.textContent).toContain('scan_started');
    expect(c.textContent).toContain('task_completed');
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('says how many it is showing, and says when a full page was capped', async () => {
    const c = await mount({
      rows: [logRow({ id: 'a' }), logRow({ id: 'b' })],
      filters: { ...DEFAULT_FILTERS, limit: 2 },
      capped: true,
    });
    expect(c.querySelector('[data-log-count]')?.textContent).toContain('showing 2 logs');
    expect(c.querySelector('[data-log-capped]')?.textContent).toContain('capped at 2');
  });
});

describe('Observatory · Logs — a failed read', () => {
  it('is the error, never “no logs for these filters”', async () => {
    const c = await mount({
      rows: [],
      rowsNotice: notice("Couldn't read the log", 'daemon unreachable'),
      rowsError: 'Could not read the log: daemon unreachable',
    });
    const noticeEl = c.querySelector('[data-logs-error]');
    expect(noticeEl?.textContent).toContain('daemon unreachable');
    expect(c.textContent).not.toContain('no logs for these filters');
  });
});

describe('Observatory · Logs — the client-side search', () => {
  it('narrows the loaded rows without asking the daemon', async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [logRow({ id: 'a', message: 'scan_started' }), logRow({ id: 'b', message: 'task_completed' })],
    });
    const search = c.querySelector('[data-log-search]') as HTMLInputElement;
    search.value = 'scan';
    search.dispatchEvent(new Event('input'));
    await tick();
    await vi.waitFor(() => expect(rowHeaders(c).length).toBe(1));
    expect(c.textContent).toContain('scan_started');
    expect(c.textContent).not.toContain('task_completed');
    expect(c.querySelector('[data-log-count]')?.textContent).toContain('showing 1 of 2 loaded');
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

describe('Observatory · Logs — the server-side filters', () => {
  it('navigates with the new query instead of filtering in place', async () => {
    const c = await mount({ rows: [logRow({ message: 'still here' })] });
    const level = c.querySelector('[data-log-filter-level]') as HTMLSelectElement;
    level.value = 'error';
    level.dispatchEvent(new Event('change', { bubbles: true }));
    await tick();
    expect(vi.mocked(goto)).toHaveBeenCalledTimes(1);
    const [href, opts] = vi.mocked(goto).mock.calls[0];
    expect(href).toBe('/activity-logs?level=error');
    expect(opts).toMatchObject({ invalidateAll: true, keepFocus: true, noScroll: true });
  });
});

describe('Observatory · Logs — a row’s payload', () => {
  it('opens on the row and closes again', async () => {
    const c = await mount({ rows: [logRow({ id: 'a', context: { folder: 'Developer' } })] });
    expect(c.querySelector('[data-log-payload]')).toBeNull();
    (c.querySelector('[data-log-row-header]') as HTMLButtonElement).click();
    await tick();
    expect(c.querySelector('[data-log-payload]')?.textContent).toContain('Developer');
    (c.querySelector('[data-log-row-header]') as HTMLButtonElement).click();
    await tick();
    expect(c.querySelector('[data-log-payload]')).toBeNull();
  });
});

describe('Observatory · Logs — the background tasks', () => {
  it('draws each worker’s health, and why a failing one is failing', async () => {
    const c = await mount({
      tasks: [
        task({ name: 'scanner' }),
        task({ name: 'watchdog', last_ok: false, last_error: 'timeout reading ~/.sensei' }),
      ],
    });
    const rows = c.querySelectorAll('[data-testid="scheduled-task"]');
    expect(rows.length).toBe(2);
    const watchdog = c.querySelector('[data-task-name="watchdog"]') as HTMLElement;
    expect(watchdog.querySelector('[data-task-health]')?.textContent?.trim()).toBe('failing');
    expect(watchdog.querySelector('[data-task-error]')?.textContent).toContain('timeout reading ~/.sensei');
    expect(c.querySelector('[data-task-name="scanner"] [data-task-health]')?.textContent?.trim()).toBe('ok');
  });

  it('a failed tasks read is the error, not an empty panel', async () => {
    const c = await mount({
      tasks: [],
      tasksNotice: notice("Couldn't read the background tasks", 'daemon unreachable'),
      tasksError: 'Could not read the background tasks: daemon unreachable',
    });
    expect(c.querySelector('[data-tasks-error]')?.textContent).toContain('daemon unreachable');
    expect(c.querySelector('[data-testid="scheduled-tasks"]')).toBeNull();
  });
});

// Phase B (R1): the count strip is a claim about the log's contents. A read
// that failed has no count — "showing 0 logs" beside an error notice says the
// log is empty when nobody could read it (D83).
describe('Observatory · Logs — a failed read does not count rows', () => {
  it.skip('pending: L5 activity-logs — no row count from a failed read', async () => {
    const c = await mount({
      rows: [],
      rowsNotice: notice("Couldn't read the log", 'daemon unreachable'),
      rowsError: 'Could not read the log: daemon unreachable',
    });
    expect(c.querySelector('[data-logs-error]')?.textContent).toContain('daemon unreachable');
    expect(c.querySelector('[data-log-count]')).toBeNull();
    expect(c.textContent).not.toContain('showing 0 logs');
  });
});

// Phase B (R3): the log read's own failure gets the same next step. The notice
// says "Retry" today with nothing to press, so a daemon hiccup is a dead end
// until the page is reloaded — the filters the reader chose must survive it.
describe('Observatory · Logs — a failed log read can be retried', () => {
  it.skip('pending: L5 activity-logs — the log notice re-reads the rows with the same filters', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => [logRow({ message: 'back again' })],
    });
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [],
      filters: { ...DEFAULT_FILTERS, level: 'error', since: '1h' },
      rowsNotice: notice("Couldn't read the log", 'daemon unreachable'),
      rowsError: 'Could not read the log: daemon unreachable',
    });

    const retry = c.querySelector('[data-logs-error] [data-action="retry"]') as HTMLButtonElement;
    expect(retry).not.toBeNull();
    retry.click();

    await vi.waitFor(() => expect(rowHeaders(c).length).toBe(1));
    expect(c.textContent).toContain('back again');
    expect(c.querySelector('[data-logs-error]')).toBeNull();
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    const url = String(fetchSpy.mock.calls[0][0]);
    expect(url).toContain('/api/logs');
    expect(url).toContain('level=error');
    expect(url).toContain('since=1h');
  });
});

// Phase B (R4): the retry's own answer decides what is drawn. A re-read that
// fails again is the new error, not "no logs for these filters" and not the
// stale reason — a failed retry that reads as an empty log is the same D83
// claim as the count strip (settings/extensions tests r1, M2).
describe('Observatory · Logs — a log retry that fails again', () => {
  it.skip('pending: L5 activity-logs — a failed log retry is the new reason, not an empty log', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 502, statusText: 'Bad Gateway' });
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [],
      rowsNotice: notice("Couldn't read the log", 'daemon unreachable'),
      rowsError: 'Could not read the log: daemon unreachable',
    });

    const retry = c.querySelector('[data-logs-error] [data-action="retry"]') as HTMLButtonElement;
    expect(retry).not.toBeNull();
    retry.click();

    await vi.waitFor(() => {
      expect(c.querySelector('[data-logs-error]')).not.toBeNull();
      expect(c.querySelector('[data-logs-error]')?.textContent).toContain('Bad Gateway');
    });
    expect(c.querySelector('[data-logs-error]')?.textContent).not.toContain('daemon unreachable');
    expect(c.textContent).not.toContain('no logs for these filters');
    expect(c.querySelector('[data-log-count]')).toBeNull();
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });
});

// Phase B (R5): the same for the tasks — a panel that quietly disappears on a
// failed retry hides the failure just as an empty log does.
describe('Observatory · Logs — a tasks retry that fails again', () => {
  it.skip('pending: L5 activity-logs — a failed tasks retry is the new reason, not a missing panel', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 502, statusText: 'Bad Gateway' });
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [logRow({ message: 'still here' })],
      tasks: [],
      tasksNotice: notice("Couldn't read the background tasks", 'daemon unreachable'),
      tasksError: 'Could not read the background tasks: daemon unreachable',
    });

    const retry = c.querySelector('[data-tasks-error] [data-action="retry"]') as HTMLButtonElement;
    expect(retry).not.toBeNull();
    retry.click();

    await vi.waitFor(() => {
      expect(c.querySelector('[data-tasks-error]')).not.toBeNull();
      expect(c.querySelector('[data-tasks-error]')?.textContent).toContain('Bad Gateway');
    });
    expect(c.querySelector('[data-tasks-error]')?.textContent).not.toContain('daemon unreachable');
    expect(c.querySelector('[data-testid="scheduled-tasks"]')).toBeNull();
    expect(c.textContent).toContain('still here');
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });
});

// Phase B (R6): a retry in flight is not a stampede — the second press during
// the read joins it instead of starting another (the `reading` guard).
describe('Observatory · Logs — a retry pressed twice', () => {
  it.skip('pending: L5 activity-logs — two presses of the log retry are one GET', async () => {
    let answer: (v: unknown) => void = () => {};
    const pending = new Promise((resolve) => {
      answer = resolve;
    });
    const fetchSpy = vi.fn().mockReturnValue(pending);
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [],
      rowsNotice: notice("Couldn't read the log", 'daemon unreachable'),
      rowsError: 'Could not read the log: daemon unreachable',
    });

    const retry = c.querySelector('[data-logs-error] [data-action="retry"]') as HTMLButtonElement;
    expect(retry).not.toBeNull();
    retry.click();
    retry.click();

    answer({ ok: true, status: 200, json: async () => [logRow({ message: 'back again' })] });
    await vi.waitFor(() => expect(rowHeaders(c).length).toBe(1));
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });
});

// Phase B (R7, D87): the page adopts the shell — the title bar, the filter band
// (with the search), and a body and a sidebar under `main` that scroll on their
// own. The workers sit in the sidebar so the rows can scroll without them
// (#293): "the header, the workers, the filters and the column headings stay in
// place; the rows scroll".
describe('Observatory · Logs — the shell (D87)', () => {
  it.skip('pending: L5 activity-logs — the shell draws the header, the band and the two scrollers', async () => {
    const c = await mount({
      rows: [logRow({ message: 'still here' })],
      tasks: [task({ name: 'scanner' })],
    });

    const shell = c.querySelector('[data-screen-shell]');
    expect(shell).not.toBeNull();
    expect(shell?.getAttribute('data-view')).toBe('observatory.logs');
    expect(c.querySelector('[data-screen-title]')?.textContent?.trim()).toBe('Activity logs');
    expect(c.querySelector('[data-screen-context]')?.textContent).toContain('Observatory · Logs');
    expect(c.querySelector('[data-screen-mark]')).not.toBeNull();
    expect(c.querySelector('[data-screen-description]')?.textContent).toContain(
      'Daemon, cli, mcp and app activity',
    );

    // The band: the server-side controls, and the shell's search beside them —
    // one search, not the shell's plus a page copy (tests r2, L2).
    const band = c.querySelector('[data-screen-filters]') as HTMLElement;
    expect(band).not.toBeNull();
    expect(band.querySelector('[data-log-filter-level]')).not.toBeNull();
    expect(band.querySelector('[data-screen-search] input')).not.toBeNull();
    expect(band.querySelectorAll('input[type="search"]').length).toBe(1);

    // Two scrollers: the body (the rows) and the sidebar (the workers).
    const main = c.querySelector('[data-screen-main]') as HTMLElement;
    expect(main).not.toBeNull();
    const body = main.querySelector('[data-screen-body]') as HTMLElement;
    const side = main.querySelector('[data-screen-sidebar]') as HTMLElement;
    expect(body?.parentElement).toBe(main);
    expect(side?.parentElement).toBe(main);
    expect(side?.getAttribute('data-side')).toBe('right');
    expect(body.querySelector('[data-log-row-header]')).not.toBeNull();
    expect(side.querySelector('[data-testid="scheduled-tasks"]')).not.toBeNull();

    // And it is rokkit's shell, not a lookalike, rendered for this view (L1).
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    expect(vi.mocked(ScreenShell).mock.calls[0][1]).toMatchObject({ viewId: 'observatory.logs' });
  });
});

// Phase B (R8, N1): the tasks Retry writes the same guard as the log one — a
// second press during the read joins it rather than starting another.
describe('Observatory · Logs — a tasks retry pressed twice', () => {
  it.skip('pending: L5 activity-logs — two presses of the tasks retry are one GET', async () => {
    let answer: (v: unknown) => void = () => {};
    const pending = new Promise((resolve) => {
      answer = resolve;
    });
    const fetchSpy = vi.fn().mockReturnValue(pending);
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      tasks: [],
      tasksNotice: notice("Couldn't read the background tasks", 'daemon unreachable'),
      tasksError: 'Could not read the background tasks: daemon unreachable',
    });

    const retry = c.querySelector('[data-tasks-error] [data-action="retry"]') as HTMLButtonElement;
    expect(retry).not.toBeNull();
    retry.click();
    retry.click();

    answer({ ok: true, status: 200, json: async () => ({ tasks: [task({ name: 'scanner' })] }) });
    await vi.waitFor(() => expect(c.querySelector('[data-task-name="scanner"]')).not.toBeNull());
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });
});

// Phase B (R2): a tasks read that failed is retryable where it failed — the
// log rows stay, one request goes out, and the answer replaces the notice.
describe('Observatory · Logs — a failed tasks read can be retried', () => {
  it.skip('pending: L5 activity-logs — the tasks notice re-reads the tasks in place', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ tasks: [task({ name: 'scanner' })] }),
    });
    vi.stubGlobal('fetch', fetchSpy);
    const c = await mount({
      rows: [logRow({ message: 'still here' })],
      tasks: [],
      tasksNotice: notice("Couldn't read the background tasks", 'daemon unreachable'),
      tasksError: 'Could not read the background tasks: daemon unreachable',
    });

    const retry = c.querySelector('[data-tasks-error] [data-action="retry"]') as HTMLButtonElement;
    expect(retry).not.toBeNull();
    retry.click();

    await vi.waitFor(() => expect(c.querySelector('[data-task-name="scanner"]')).not.toBeNull());
    expect(c.querySelector('[data-tasks-error]')).toBeNull();
    expect(c.textContent).toContain('still here');
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(String(fetchSpy.mock.calls[0][0])).toContain('/api/tasks/scheduled');
  });
});
