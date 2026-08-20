// Sessions digest — pure derivations (no runes).
//
// Everything the digest draws is computed here from the wire `SessionRow`s so
// the state class and components stay thin. Kept framework-free so quality
// mapping, per-day aggregation, and the row display strings are unit-testable
// without mounting anything or standing up the daemon.
//
// Shared by the Observatory Sessions screen and the project-scoped Sessions
// screen — promoted out of (observatory)/sessions/ once a second consumer
// appeared (frontend-svelte-guidelines §DRY).
//
// Screens: docs/spec/screen/observatory-sessions.md
//          docs/spec/screen/project-sessions.md
// Mockup:  docs/mockups/Sensei/lib/sessions-zen.jsx → SessionsDigestZen
import type { SessionRow } from '$lib/types.js';

/** The chart chips (pulse is deliberately NOT here — it is a mini-cycler-only
 *  mode, see MINI_MODES). `tokens` is the per-day token-volume bars. */
export type ChartVariant = 'trend' | 'stream' | 'constellation' | 'bands' | 'tokens';
export const CHART_VARIANTS: ChartVariant[] = ['trend', 'stream', 'constellation', 'bands', 'tokens'];

/** The collapsed-header mini-cycler modes. `numbers` is the resting stat trio;
 *  `pulse` lives ONLY here — it is never one of the CHART_VARIANTS chips. */
export type MiniMode = 'numbers' | 'trend' | 'stream' | 'constellation' | 'bands' | 'pulse';
export const MINI_MODES: MiniMode[] = ['numbers', 'trend', 'stream', 'constellation', 'bands', 'pulse'];

/** Time-window chips. Each refetches `/api/sessions?range=`. */
export type SessionRange = '7d' | '30d' | '90d';
export const SESSION_RANGES: SessionRange[] = ['7d', '30d', '90d'];

/** Three quality tones plus a neutral for the edge outcomes. */
export type QualityTone = 'good' | 'bad' | 'ugly' | 'neutral';

/** Advance the mini-cycler one step, wrapping past `pulse` back to `numbers`. */
export function nextMiniMode(mode: MiniMode): MiniMode {
  const i = MINI_MODES.indexOf(mode);
  return MINI_MODES[(i + 1) % MINI_MODES.length];
}

/** How many calendar days the day-axis spans for a range. */
export function rangeDays(range: SessionRange): number {
  return range === '7d' ? 7 : range === '30d' ? 30 : 90;
}

/**
 * Quality tone for one session, per the spec's outcome→quality mapping:
 *   good   = completed AND ftr
 *   bad    = corrected
 *   ugly   = abandoned
 *   neutral = blocked / partial (and any completed-without-ftr / unknown)
 */
export function quality(s: Pick<SessionRow, 'outcome' | 'ftr'>): QualityTone {
  if (s.outcome === 'abandoned') return 'ugly';
  if (s.outcome === 'corrected') return 'bad';
  if (s.outcome === 'completed' && s.ftr === true) return 'good';
  return 'neutral';
}

/** Human title for a row — never blank. Task first, then summary, then a
 *  generic word so the middle column never collapses (mirrors #31/#61). */
export function sessionTitle(s: Pick<SessionRow, 'task' | 'summary'>): string {
  return s.task?.trim() || s.summary?.trim() || 'session';
}

/** Duration in whole minutes, or null when the session isn't completable. */
export function durationMinutes(startedAt: string, completedAt: string | null): number | null {
  if (!completedAt) return null;
  const start = Date.parse(startedAt);
  const end = Date.parse(completedAt);
  if (Number.isNaN(start) || Number.isNaN(end) || end <= start) return null;
  return Math.round((end - start) / 60_000);
}

/** Compact duration label: "12m" / "1h 04m" / "2h" / "—". */
export function compactDuration(mins: number | null): string {
  if (mins == null) return '—';
  if (mins < 60) return `${mins}m`;
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return m === 0 ? `${h}h` : `${h}h ${String(m).padStart(2, '0')}m`;
}

/** Compact token count: "820" / "7.7k" / "1.3M" / "—". Keeps the row legible
 *  where a raw grouped integer (132,332,559) would blow out the column. */
export function compactTokens(tokens: number | null | undefined): string {
  if (tokens == null || Number.isNaN(tokens)) return '—';
  const n = Math.round(tokens);
  if (n < 1_000) return `${n}`;
  if (n < 1_000_000) return `${(n / 1_000).toFixed(n < 10_000 ? 1 : 0)}k`;
  return `${(n / 1_000_000).toFixed(n < 10_000_000 ? 1 : 0)}M`;
}

/** Active-work minutes from the daemon's gap-aware `durationSecs`, or null when
 *  the session carries no measured duration. Distinct from `durationMinutes`,
 *  which is wall-clock (startedAt→completedAt). */
export function activeMinutes(durationSecs: number | null | undefined): number | null {
  if (durationSecs == null || Number.isNaN(durationSecs) || durationSecs < 0) return null;
  return Math.round(durationSecs / 60);
}

/** HH:MM (24h) of the start time, in the viewer's locale/timezone. */
export function timeOfDay(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false });
}

/** Local calendar-day key (YYYY-MM-DD) for an ISO timestamp. */
export function dayKey(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return dayKeyFromDate(d);
}

function dayKeyFromDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** Whole calendar days from `then` up to `now` (0 = same day, 1 = yesterday). */
function calendarDaysAgo(then: Date, now: Date): number {
  const a = new Date(then.getFullYear(), then.getMonth(), then.getDate()).getTime();
  const b = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  return Math.round((b - a) / 86_400_000);
}

/** Relative label for a start time: "today" / "yesterday" / "3 days ago" /
 *  "last week" / "2 weeks ago". `now` is injectable for deterministic tests. */
export function relativeWhen(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) return '';
  const days = calendarDaysAgo(then, now);
  if (days <= 0) return 'today';
  if (days === 1) return 'yesterday';
  if (days < 7) return `${days} days ago`;
  if (days < 14) return 'last week';
  return `${Math.floor(days / 7)} weeks ago`;
}

/** Short axis label for a day key relative to today: "Today" / "Yest" / "−N". */
export function shortDayLabel(key: string, now: Date = new Date()): string {
  if (!key) return '';
  const [y, m, d] = key.split('-').map(Number);
  const then = new Date(y, (m ?? 1) - 1, d ?? 1);
  const days = calendarDaysAgo(then, now);
  if (days <= 0) return 'Today';
  if (days === 1) return 'Yest';
  return `−${days}`;
}

/** A wire row plus every derived field the digest renders. */
export interface EnrichedSession {
  id: string;
  project: string;
  title: string;
  agent: string | null;
  outcome: string;
  ftr: boolean | null;
  turns: number;
  corrections: number;
  startedAt: string;
  completedAt: string | null;
  quality: QualityTone;
  mins: number | null;
  dayKey: string;
  when: string;
  time: string;
  duration: string;
  /** Folder role for multi-repo projects (`web` / `backend` / `docs`); null
   *  when the endpoint doesn't carry it (single-folder projects, or the
   *  observatory multi-project view). Drives the row's folder-role chip. */
  folderRole: string | null;
  /** Transcript-captured token usage (null when the source carried none — never
   *  a fabricated 0). `tokens` is the in+out sum used by the row + charts. */
  tokensIn: number | null;
  tokensOut: number | null;
  tokens: number | null;
  /** Gap-aware ACTIVE work minutes from the daemon `durationSecs` (distinct from
   *  `mins`, which is wall-clock). Null when the session has no measured duration. */
  activeMins: number | null;
  /** Compact display strings for the row. */
  tokensLabel: string;
  activeLabel: string;
  /** Inference model that ran the session (e.g. "claude-opus-4-8"); null when
   *  the source didn't record one. */
  model: string | null;
}

/** Shape raw wire rows into the display contract. `now` injectable for tests. */
export function enrich(rows: SessionRow[], now: Date = new Date()): EnrichedSession[] {
  return rows.map((s) => {
    const mins = durationMinutes(s.startedAt, s.completedAt);
    const tokensIn = s.tokensIn ?? null;
    const tokensOut = s.tokensOut ?? null;
    // in+out only when at least one is present; both absent ⇒ null (honest, not 0).
    const tokens = tokensIn == null && tokensOut == null ? null : (tokensIn ?? 0) + (tokensOut ?? 0);
    const activeMins = activeMinutes(s.durationSecs);
    return {
      id: s.id,
      project: (s.project ?? '').trim(),
      title: sessionTitle(s),
      agent: s.agent,
      outcome: s.outcome,
      ftr: s.ftr ?? null,
      turns: s.turns,
      corrections: s.corrections,
      startedAt: s.startedAt,
      completedAt: s.completedAt,
      quality: quality(s),
      mins,
      dayKey: dayKey(s.startedAt),
      when: relativeWhen(s.startedAt, now),
      time: timeOfDay(s.startedAt),
      duration: compactDuration(mins),
      folderRole: s.folderRole?.trim() ? s.folderRole.trim() : null,
      tokensIn,
      tokensOut,
      tokens,
      activeMins,
      tokensLabel: compactTokens(tokens),
      activeLabel: compactDuration(activeMins),
      model: s.model?.trim() ? s.model.trim() : null,
    };
  });
}

/** Ordered oldest→newest day keys covering the range, ending today. */
export function buildDays(range: SessionRange, now: Date = new Date()): string[] {
  const n = rangeDays(range);
  const base = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const out: string[] = [];
  for (let i = n - 1; i >= 0; i--) {
    const d = new Date(base);
    d.setDate(base.getDate() - i);
    out.push(dayKeyFromDate(d));
  }
  return out;
}

/** Per-day rollup used by the trend / stream / bands charts. */
export interface DayBucket {
  day: string;
  label: string;
  total: number;
  good: number;
  bad: number;
  ugly: number;
  neutral: number;
  mins: number;
  goodMins: number;
  badMins: number;
  uglyMins: number;
  /** Minutes actually drawn in the quality charts = good + bad + ugly. Excludes
   *  neutral (blocked/partial) sessions, which have no quality tone/segment — so
   *  chart scales and per-day captions agree with the rendered bars/areas. */
  chartedMins: number;
  /** first-try rate for the day (good / total), or null when the day is empty. */
  ftr: number | null;
  /** Token volume for the day (sum over sessions carrying usage). `tokens` is
   *  in+out; the split feeds the input/output balance. 0 when no session that day
   *  carried tokens — a real absence at day grain, safe to draw as an empty bar. */
  tokensIn: number;
  tokensOut: number;
  tokens: number;
}

/** Bucket enriched sessions into the ordered `days` axis by calendar day. */
export function aggregateByDay(
  sessions: EnrichedSession[],
  days: string[],
  now: Date = new Date(),
): DayBucket[] {
  return days.map((day) => {
    const ofDay = sessions.filter((s) => s.dayKey === day);
    const by = (tone: QualityTone) => ofDay.filter((s) => s.quality === tone);
    const good = by('good');
    const bad = by('bad');
    const ugly = by('ugly');
    const neutral = by('neutral');
    const sumMins = (xs: EnrichedSession[]) => xs.reduce((a, s) => a + (s.mins ?? 0), 0);
    const tokensIn = ofDay.reduce((a, s) => a + (s.tokensIn ?? 0), 0);
    const tokensOut = ofDay.reduce((a, s) => a + (s.tokensOut ?? 0), 0);
    const total = ofDay.length;
    return {
      day,
      label: shortDayLabel(day, now),
      total,
      good: good.length,
      bad: bad.length,
      ugly: ugly.length,
      neutral: neutral.length,
      mins: sumMins(ofDay),
      goodMins: sumMins(good),
      badMins: sumMins(bad),
      uglyMins: sumMins(ugly),
      chartedMins: sumMins(good) + sumMins(bad) + sumMins(ugly),
      ftr: total ? good.length / total : null,
      tokensIn,
      tokensOut,
      tokens: tokensIn + tokensOut,
    };
  });
}

/** Median of a numeric list (0 for empty). */
export function median(xs: number[]): number {
  if (xs.length === 0) return 0;
  const sorted = [...xs].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** The header stat strip + quality tally over the whole visible slice. */
export interface DigestTotals {
  count: number;
  projects: number;
  good: number;
  bad: number;
  ugly: number;
  neutral: number;
  medianMins: number;
  /** Total tokens over sessions carrying usage, and the per-session median.
   *  Null when NO visible session carried tokens (honest — not a 0 stat). */
  totalTokens: number | null;
  medianTokens: number | null;
}

export function computeTotals(sessions: EnrichedSession[]): DigestTotals {
  const tone = (t: QualityTone) => sessions.filter((s) => s.quality === t).length;
  const projects = new Set(sessions.map((s) => s.project).filter(Boolean)).size;
  const durations = sessions.map((s) => s.mins).filter((m): m is number => m != null);
  const tokenVals = sessions.map((s) => s.tokens).filter((t): t is number => t != null);
  return {
    count: sessions.length,
    projects,
    good: tone('good'),
    bad: tone('bad'),
    ugly: tone('ugly'),
    neutral: tone('neutral'),
    medianMins: Math.round(median(durations)),
    totalTokens: tokenVals.length ? tokenVals.reduce((a, t) => a + t, 0) : null,
    medianTokens: tokenVals.length ? Math.round(median(tokenVals)) : null,
  };
}

/** Deep-link target for a row → Instruments Replay tab, session preselected.
 *  Session-id resolves via GET /api/sessions/{id} (shipped Slot 1). */
export function replayHref(sessionId: string): string {
  return `/instruments?tab=replay&session=${encodeURIComponent(sessionId)}`;
}
