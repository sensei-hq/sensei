/** Color maps — single source of truth for all status/category badge colors. */

export const STATUS_CLS: Record<string, string> = {
  active: 'bg-success-z2 text-success-z7',
  recent: 'bg-primary-z2 text-primary-z7',
  stale: 'bg-warning-z2 text-warning-z7',
  archived: 'bg-surface-z3 text-surface-z5',
  abandoned: 'bg-error-z2 text-error-z7',
  unknown: 'bg-surface-z3 text-surface-z5',
};

export const INDEX_STATUS_CLS: Record<string, string> = {
  idle: 'bg-surface-z3 text-surface-z5',
  queued: 'bg-info-z2 text-info-z7',
  indexing: 'bg-primary-z2 text-primary-z7',
  indexed: 'bg-success-z2 text-success-z7',
  failed: 'bg-error-z2 text-error-z7',
};

export const ROLE_CLS: Record<string, string> = {
  backend: 'bg-info-z2 text-info-z7',
  frontend: 'bg-accent-z2 text-accent-z7',
  mobile: 'bg-secondary-z2 text-secondary-z7',
  docs: 'bg-warning-z2 text-warning-z7',
  infra: 'bg-surface-z3 text-surface-z6',
  library: 'bg-primary-z2 text-primary-z7',
  shared: 'bg-surface-z3 text-surface-z6',
  monorepo: 'bg-primary-z2 text-primary-z7',
  reference: 'bg-surface-z3 text-surface-z5',
  unknown: 'bg-surface-z3 text-surface-z5',
};

export const CAT_CLS: Record<string, string> = {
  app: 'bg-primary-z2 text-primary-z6',
  library: 'bg-info-z2 text-info-z6',
  tool: 'bg-warning-z2 text-warning-z7',
  idea: 'bg-surface-z3 text-surface-z5',
  unknown: 'bg-surface-z2 text-surface-z4',
};

export const OUTCOME_CLS: Record<string, { cls: string; icon: string; label: string }> = {
  completed: { cls: 'bg-success-z2 text-success-z7', icon: '✓', label: 'completed' },
  partial: { cls: 'bg-warning-z2 text-warning-z7', icon: '△', label: 'partial' },
  blocked: { cls: 'bg-error-z2 text-error-z7', icon: '✗', label: 'blocked' },
};

export const OUTCOME_DEFAULT = { cls: 'bg-surface-z3 text-surface-z5', icon: '·', label: '—' };

export function outcomeBadge(outcome: string | null) {
  return (outcome && OUTCOME_CLS[outcome]) ?? OUTCOME_DEFAULT;
}

export function ftrColorClass(ftr: number | null | undefined): string {
  if (ftr == null) return 'text-surface-z4';
  if (ftr >= 0.8) return 'text-success-z6';
  if (ftr >= 0.5) return 'text-warning-z6';
  return 'text-error-z6';
}

export function ftrFormat(ftr: number | null | undefined): string {
  if (ftr == null) return '—';
  return `${Math.round(ftr * 100)}%`;
}
