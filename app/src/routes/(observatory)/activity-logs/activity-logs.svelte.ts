// Observatory · Logs — pure derivation + the screen's small reactive view.
//
// The operator surface: daemon / cli / mcp / app activity, read from
// `GET /api/logs`. Level / source / module / since / limit are server query
// params (the loader reads them from the URL, the controls refetch); the free
// text search and which row's payload is expanded are client-only state.
//
// Everything here is pure and unit-tested except `ActivityLogsView`, the tiny
// mutable surface the search box and row toggles bind to.

import type { LogRow, LogLevel } from '$lib/types.js';

/** Known severity levels, most→least severe last for the picker order. */
export const LEVELS: LogLevel[] = ['trace', 'debug', 'info', 'warn', 'error'];

/** Time-range chips. `all` maps to the endpoint's no-lower-bound. */
export const SINCE_OPTIONS = [
  { label: '15m', value: '15m' },
  { label: '1h', value: '1h' },
  { label: '24h', value: '24h' },
  { label: 'All', value: 'all' },
] as const;

/** Row-cap choices. Ceiling mirrors the daemon's `MAX_LOG_LIMIT`. */
export const LIMIT_OPTIONS = [50, 100, 200, 500, 1000] as const;
export const DEFAULT_LIMIT = 200;
export const MAX_LIMIT = 1000;
export const DEFAULT_SINCE = 'all';

/** Server-side filter state — drives the loader refetch via the URL. Empty
 *  string means "no filter" (the control's "All" option). */
export interface LogFilters {
  level: string;
  source: string;
  module: string;
  since: string;
  limit: number;
}

export const DEFAULT_FILTERS: LogFilters = {
  level: '',
  source: '',
  module: '',
  since: DEFAULT_SINCE,
  limit: DEFAULT_LIMIT,
};

/** Read the loader filters from the page URL. Unknown / out-of-range values
 *  fall back to the defaults; `limit` is clamped to `[1, MAX_LIMIT]`. */
export function filtersFromParams(params: URLSearchParams): LogFilters {
  const rawLimit = Number(params.get('limit'));
  const limit =
    Number.isFinite(rawLimit) && rawLimit > 0
      ? Math.min(Math.trunc(rawLimit), MAX_LIMIT)
      : DEFAULT_LIMIT;
  return {
    level: params.get('level') ?? '',
    source: params.get('source') ?? '',
    module: params.get('module') ?? '',
    since: params.get('since') ?? DEFAULT_SINCE,
    limit,
  };
}

/** Build the query string for `goto()` from a filter set. Omits empty filters
 *  and any value equal to its default so the URL stays clean; returns `''`
 *  when everything is at its default (a bare pathname). */
export function buildLogsQuery(filters: LogFilters): string {
  const p = new URLSearchParams();
  if (filters.level) p.set('level', filters.level);
  if (filters.source) p.set('source', filters.source);
  if (filters.module) p.set('module', filters.module);
  if (filters.since && filters.since !== DEFAULT_SINCE) p.set('since', filters.since);
  if (filters.limit !== DEFAULT_LIMIT) p.set('limit', String(filters.limit));
  const qs = p.toString();
  return qs ? `?${qs}` : '';
}

/** Severity styling — maps a level to rokkit role-token utility classes.
 *  Unknown levels fall back to the neutral ink ramp. */
export function levelTone(level: string): { text: string; bg: string } {
  switch (level) {
    case 'error':
      return { text: 'text-error', bg: 'bg-error-soft' };
    case 'warn':
      return { text: 'text-warning', bg: 'bg-warning-soft' };
    case 'info':
      return { text: 'text-info', bg: 'bg-info-soft' };
    case 'debug':
      return { text: 'text-ink-soft', bg: 'bg-paper-mute' };
    case 'trace':
      return { text: 'text-ink-faint', bg: 'bg-paper-mute' };
    default:
      return { text: 'text-ink-mute', bg: 'bg-paper-mute' };
  }
}

/** The row's `module` (a first-class `public.logs` column), or null when absent. */
export function moduleOf(row: LogRow): string | null {
  const m = row.module;
  return typeof m === 'string' && m ? m : null;
}

/** Human relative time, e.g. `just now`, `2m ago`, `3h ago`, `5d ago`.
 *  Falls back to the raw string when the timestamp doesn't parse. */
export function relativeTime(nowMs: number, isoTs: string): string {
  const t = Date.parse(isoTs);
  if (Number.isNaN(t)) return isoTs;
  const secs = Math.max(0, Math.round((nowMs - t) / 1000));
  if (secs < 5) return 'just now';
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

/** Absolute local timestamp for the row title / tooltip. */
export function absoluteTime(isoTs: string): string {
  const t = Date.parse(isoTs);
  if (Number.isNaN(t)) return isoTs;
  return new Date(t).toLocaleString();
}

/** Case-insensitive match of a query against the message, module and source. */
export function matchesSearch(row: LogRow, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const hay = [row.message, moduleOf(row) ?? '', row.source ?? '']
    .join(' ')
    .toLowerCase();
  return hay.includes(q);
}

/** Client-side text filter over a row list. An empty query returns the input
 *  unchanged (same reference). */
export function searchRows(rows: LogRow[], query: string): LogRow[] {
  if (!query.trim()) return rows;
  return rows.filter((r) => matchesSearch(r, query));
}

/** True when a row carries any non-empty payload worth expanding. */
export function hasPayload(row: LogRow): boolean {
  return payloadSections(row).length > 0;
}

/** The non-empty payload sections of a row, each pretty-printed. `error` is
 *  first so failures lead; empty objects and nulls are dropped — nothing is
 *  truncated. */
export function payloadSections(row: LogRow): { label: string; json: string }[] {
  const out: { label: string; json: string }[] = [];
  const push = (label: string, v: unknown) => {
    if (v == null) return;
    if (typeof v === 'object' && Object.keys(v as object).length === 0) return;
    out.push({ label, json: JSON.stringify(v, null, 2) });
  };
  push('error', row.error);
  push('context', row.context);
  push('data', row.data);
  return out;
}

/** Distinct non-null `source` values across the rows, sorted, unioned with the
 *  active selection so a server-narrowed page keeps the current filter
 *  selectable. */
export function distinctSources(rows: LogRow[], active: string): string[] {
  const set = new Set<string>();
  for (const r of rows) if (r.source) set.add(r.source);
  if (active) set.add(active);
  return [...set].sort();
}

/** Distinct `module` values across the rows (see `distinctSources`). */
export function distinctModules(rows: LogRow[], active: string): string[] {
  const set = new Set<string>();
  for (const r of rows) {
    const m = moduleOf(r);
    if (m) set.add(m);
  }
  if (active) set.add(active);
  return [...set].sort();
}

/** Client-only view state: the search box + which row's payload is expanded.
 *  All server filters live in the URL, not here. */
export class ActivityLogsView {
  search = $state('');
  expandedId = $state<string | null>(null);

  toggle(id: string): void {
    this.expandedId = this.expandedId === id ? null : id;
  }

  clear(): void {
    this.expandedId = null;
  }
}
