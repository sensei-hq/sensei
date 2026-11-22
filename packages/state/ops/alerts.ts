// The alert severity mapping: every failure class joins one scale — a level
// (info/warn/critical), a reason kind (normal/refusal/fault, the
// `sensei.reason_kind` axis), and a reason code. Pure, so the store, the
// section, and the toast wiring all read the same answer.

export type AlertClass = 'task' | 'schedule' | 'sync' | 'stream' | 'daemon';
export type AlertLevel = 'info' | 'warn' | 'critical';
export type AlertKind = 'normal' | 'refusal' | 'fault';

export interface AlertSeverity {
  level: AlertLevel;
  kind: AlertKind;
  code: string;
}

export interface OpsAlert extends AlertSeverity {
  class: AlertClass;
  key: string;
  title: string;
  detail: string;
  /** Consecutive refreshes standing — the dedupe count, not separate alerts. */
  count: number;
  firstSeen: number;
  lastSeen: number;
  acked: boolean;
  /** True when this alert recurred after clearing — one level up. */
  escalated: boolean;
}

/** What the mapping reads of a condition: the error text and the shape flags. */
export interface AlertFields {
  error?: string | null;
  abandoned?: boolean;
  indexError?: boolean;
  skipped?: boolean;
  remedy?: string | null;
}

/**
 * The severity for one condition. The watchdog prefix is the one
 * machine-readable signal failure strings carry (mirroring the daemon's
 * frame error codes); a skipped sync row with a remedy is a refusal — decided,
 * actionable, quiet — while every other condition is a fault that warns, and a
 * dead daemon is the only critical.
 */
export function classifyAlert(class_: AlertClass, fields: AlertFields): AlertSeverity {
  switch (class_) {
    case 'daemon':
      return { level: 'critical', kind: 'fault', code: 'daemon-down' };
    case 'stream':
      return { level: 'warn', kind: 'fault', code: 'stream-fault' };
    case 'schedule':
      return { level: 'warn', kind: 'fault', code: 'schedule-failing' };
    case 'sync':
      if (fields.skipped && fields.remedy) {
        return { level: 'info', kind: 'refusal', code: 'sync-skipped' };
      }
      return { level: 'warn', kind: 'fault', code: 'sync-error' };
    case 'task':
      if (fields.abandoned) return { level: 'warn', kind: 'fault', code: 'task-abandoned' };
      if (fields.indexError) return { level: 'warn', kind: 'fault', code: 'index-error' };
      if (fields.error?.startsWith('watchdog:')) {
        return { level: 'warn', kind: 'fault', code: 'watchdog-timeout' };
      }
      return { level: 'warn', kind: 'fault', code: 'task-failed' };
  }
}

const LEVEL_RANK: Record<AlertLevel, number> = { critical: 0, warn: 1, info: 2 };

/** Critical above warnings above info; newest first within a level. */
export function sortAlerts(alerts: OpsAlert[]): OpsAlert[] {
  return [...alerts].sort(
    (a, b) => LEVEL_RANK[a.level] - LEVEL_RANK[b.level] || b.lastSeen - a.lastSeen,
  );
}
