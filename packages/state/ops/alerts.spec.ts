import { describe, it, expect } from 'vitest';
import { classifyAlert, sortAlerts, type OpsAlert } from './alerts.js';

// The severity mapping joins every failure class onto one scale: a level
// (info/warn/critical), a reason kind (normal/refusal/fault), and a reason
// code. Pure — the store and the section share it.
describe('classifyAlert', () => {
  it('a failed task is a faulted warning; a watchdog kill names its code', () => {
    expect(classifyAlert('task', { error: 'parse blew up' })).toEqual({
      level: 'warn',
      kind: 'fault',
      code: 'task-failed',
    });
    expect(classifyAlert('task', { error: 'watchdog: exceeded 600s' }).code).toBe('watchdog-timeout');
  });

  it('abandonments and index errors are faulted warnings', () => {
    expect(classifyAlert('task', { abandoned: true }).code).toBe('task-abandoned');
    expect(classifyAlert('task', { indexError: true }).code).toBe('index-error');
  });

  it('a failing schedule is a faulted warning', () => {
    expect(classifyAlert('schedule', {})).toEqual({ level: 'warn', kind: 'fault', code: 'schedule-failing' });
  });

  it('a sync error faults; a skipped row with a remedy refuses quietly', () => {
    expect(classifyAlert('sync', {})).toEqual({ level: 'warn', kind: 'fault', code: 'sync-error' });
    expect(classifyAlert('sync', { skipped: true, remedy: 'ask an owner' })).toEqual({
      level: 'info',
      kind: 'refusal',
      code: 'sync-skipped',
    });
  });

  it('a stream fault warns; a dead daemon is critical', () => {
    expect(classifyAlert('stream', {})).toEqual({ level: 'warn', kind: 'fault', code: 'stream-fault' });
    expect(classifyAlert('daemon', {})).toEqual({ level: 'critical', kind: 'fault', code: 'daemon-down' });
  });
});

describe('sortAlerts', () => {
  const alert = (level: OpsAlert['level'], lastSeen: number, key = level): OpsAlert => ({
    class: 'task',
    key,
    level,
    kind: 'fault',
    code: 'task-failed',
    title: key,
    detail: '',
    count: 1,
    firstSeen: lastSeen,
    lastSeen,
    acked: false,
    escalated: false,
  });

  it('orders critical above warnings above info, newest first within a level', () => {
    const sorted = sortAlerts([
      alert('info', 3),
      alert('warn', 1, 'old'),
      alert('critical', 2),
      alert('warn', 4, 'new'),
    ]);
    expect(sorted.map((a) => a.key)).toEqual(['critical', 'new', 'old', 'info']);
  });
});
