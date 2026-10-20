import type { ObservatoryFtr } from '$lib/types.js';

/** A read as the page holds it: the data, or why it could not be read. */
type Read<T> = { data: T | null; error: string | null };

/** The Health tab's first-try KPI. A failed read says so (#288) — "—" is
 *  reserved for no rate yet, and for a read still in flight (`null`). */
export function firstTryKpi(read: Read<Pick<ObservatoryFtr, 'ftr14d'>> | null): string {
  if (read?.error) return 'unavailable';
  const rate = read?.data?.ftr14d;
  return rate == null ? '—' : `${Math.round(rate * 100)}%`;
}
