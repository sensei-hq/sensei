import { describe, it, expect } from 'vitest';
import type { ScheduledTask } from '$lib/types.js';
import {
  EM_DASH,
  NEVER,
  taskLastRun,
  taskHealth,
  taskError,
  taskNextRun,
  taskInterval,
  taskAvg,
  taskSchedule,
} from './scheduled-tasks.svelte.js';

const BASE = Date.parse('2026-07-13T02:19:29.612354+00:00');

function makeTask(over: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    name: over.name ?? 'analyzer',
    description: over.description ?? 'derives signals and insights',
    last_run_at: 'last_run_at' in over ? (over.last_run_at ?? null) : null,
    last_ok: over.last_ok ?? null,
    last_error: over.last_error ?? null,
    next_run_at: over.next_run_at ?? null,
    interval_secs: over.interval_secs ?? null,
    avg_ms: over.avg_ms ?? null,
    enabled: over.enabled ?? true,
    window: 'window' in over ? (over.window ?? null) : null,
    days: over.days ?? null,
  };
}

describe('taskSchedule', () => {
  // The three fields PATCH can set. Before schedules became data these did not
  // exist on the wire at all, so a row could not show what it was actually
  // going to do — only how often.
  it('says "off" when the schedule is disabled, whatever else is set', () => {
    // Disabling a SCHEDULE does not disable the CAPABILITY, but the row must
    // not imply the worker is still going to run.
    const task = makeTask({ enabled: false, interval_secs: 3600, window: '22:00-05:00' });
    expect(taskSchedule(task)).toBe('off');
  });

  it('is "any time" when no window and no day mask are set', () => {
    // An unset mask means EVERY day, never "never" — the row has to say so,
    // because an empty cell reads as "nothing scheduled".
    expect(taskSchedule(makeTask({ enabled: true }))).toBe('any time');
  });

  it('shows the window as the daemon labelled it, including one that wraps midnight', () => {
    expect(taskSchedule(makeTask({ window: '22:00-05:00' }))).toBe('22:00-05:00');
  });

  it('names the days rather than printing ISO numbers', () => {
    // "1,2,3,4,5" is not something a user should have to decode.
    expect(taskSchedule(makeTask({ days: [1, 2, 3, 4, 5] }))).toBe('Mon-Fri');
    expect(taskSchedule(makeTask({ days: [6, 7] }))).toBe('Sat, Sun');
    expect(taskSchedule(makeTask({ days: [3] }))).toBe('Wed');
  });

  it('combines a window and a day mask', () => {
    expect(taskSchedule(makeTask({ days: [1, 2, 3, 4, 5], window: '09:00-17:00' }))).toBe(
      'Mon-Fri 09:00-17:00'
    );
  });
});

describe('taskLastRun', () => {
  it('renders a relative time when last_run_at is set', () => {
    const task = makeTask({ last_run_at: '2026-07-13T02:19:29.612354+00:00' });
    expect(taskLastRun(BASE + 3 * 3600_000, task)).toBe('3h ago');
    expect(taskLastRun(BASE, task)).toBe('just now');
  });

  it('reads "never" when last_run_at is null', () => {
    expect(taskLastRun(BASE, makeTask({ last_run_at: null }))).toBe(NEVER);
    expect(NEVER).toBe('never');
  });

  it('falls back to the raw string for an unparseable timestamp', () => {
    expect(taskLastRun(BASE, makeTask({ last_run_at: 'not-a-date' }))).toBe('not-a-date');
  });
});

describe('null health fields render the em dash', () => {
  const task = makeTask(); // every health field null (the daemon today)

  it('health is the em dash when last_ok is null', () => {
    expect(taskHealth(task)).toBe(EM_DASH);
    expect(EM_DASH).toBe('—');
  });

  it('error is the em dash when last_error is null', () => {
    expect(taskError(task)).toBe(EM_DASH);
  });

  it('next run is the em dash when next_run_at is null', () => {
    expect(taskNextRun(BASE, task)).toBe(EM_DASH);
  });

  it('interval is the em dash when interval_secs is null', () => {
    expect(taskInterval(task)).toBe(EM_DASH);
  });

  it('avg is the em dash when avg_ms is null', () => {
    expect(taskAvg(task)).toBe(EM_DASH);
  });
});

describe('taskHealth', () => {
  it('reads "ok" / "failing" once outcomes are recorded', () => {
    expect(taskHealth(makeTask({ last_ok: true }))).toBe('ok');
    expect(taskHealth(makeTask({ last_ok: false }))).toBe('failing');
  });
});

describe('taskError', () => {
  it('passes a recorded error through verbatim', () => {
    expect(taskError(makeTask({ last_error: 'timed out' }))).toBe('timed out');
  });
});

describe('taskNextRun', () => {
  it('renders a relative time when next_run_at is set', () => {
    const task = makeTask({ next_run_at: '2026-07-13T02:19:29.612354+00:00' });
    expect(taskNextRun(BASE + 2 * 60_000, task)).toBe('2m ago');
  });
});

describe('taskInterval', () => {
  it('formats cadence in the largest whole unit', () => {
    expect(taskInterval(makeTask({ interval_secs: 30 }))).toBe('every 30s');
    expect(taskInterval(makeTask({ interval_secs: 300 }))).toBe('every 5m');
    expect(taskInterval(makeTask({ interval_secs: 7200 }))).toBe('every 2h');
    expect(taskInterval(makeTask({ interval_secs: 172800 }))).toBe('every 2d');
  });

  it('is the em dash for a non-positive interval', () => {
    expect(taskInterval(makeTask({ interval_secs: 0 }))).toBe(EM_DASH);
    expect(taskInterval(makeTask({ interval_secs: -5 }))).toBe(EM_DASH);
  });
});

describe('taskAvg', () => {
  it('renders sub-second in ms and seconds with one decimal', () => {
    expect(taskAvg(makeTask({ avg_ms: 12 }))).toBe('12ms');
    expect(taskAvg(makeTask({ avg_ms: 12.6 }))).toBe('13ms');
    expect(taskAvg(makeTask({ avg_ms: 1400 }))).toBe('1.4s');
  });

  it('is the em dash for a negative average', () => {
    expect(taskAvg(makeTask({ avg_ms: -1 }))).toBe(EM_DASH);
  });
});
