// Number formats every screen shares (audit 2026-10-28, item 15: a duration was
// formatted three ways by three private helpers). A value not measured is a
// dash — never a made-up zero.

/** "45 ms" under a second, "1.5 s" above; "—" when not measured. */
export function fmtMs(ms: number | null | undefined): string {
  if (ms == null) return '—';
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`;
}

/** "12,345"; "—" when not counted. */
export function fmtCount(n: number | null | undefined): string {
  return n == null ? '—' : n.toLocaleString('en-US');
}
