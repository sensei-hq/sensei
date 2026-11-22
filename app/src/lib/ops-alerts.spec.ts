import { describe, it, expect } from 'vitest';
import { opsAlertToast } from './ops-alerts.js';
import type { OpsAlert } from '@sensei/state';

const alert = (level: OpsAlert['level']): OpsAlert => ({
  class: 'task',
  key: '7',
  level,
  kind: 'fault',
  code: 'task-failed',
  title: 'process_file failed',
  detail: 'boom',
  count: 1,
  firstSeen: 1,
  lastSeen: 1,
  acked: false,
  escalated: false,
});

describe('opsAlertToast', () => {
  it('a warning toasts for 8s and names the way back', () => {
    expect(opsAlertToast(alert('warn'))).toEqual({
      type: 'warning',
      text: 'process_file failed — boom (Operations)',
      timeout: 8000,
    });
  });

  it('a critical toasts as an error for 15s', () => {
    const toast = opsAlertToast(alert('critical'));
    expect(toast.type).toBe('error');
    expect(toast.timeout).toBe(15000);
  });
});
