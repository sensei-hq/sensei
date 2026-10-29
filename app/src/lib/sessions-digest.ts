// Sessions digest — pure derivations (no runes).
//
// Everything the digest draws is computed here from the wire `SessionRow`s so
// the state class and components stay thin. Kept framework-free so quality
// mapping, per-day aggregation, and the row display strings are unit-testable
// without mounting anything or standing up the daemon.
//
// Shared by the Observatory Sessions screen and the workspace-scoped Sessions
// screen — promoted out of (observatory)/sessions/ once a second consumer
// appeared (frontend-svelte-guidelines §DRY).
//
// Screens: docs/spec/screen/observatory-sessions.md
//          docs/spec/screen/workspace-sessions.md
// Mockup:  docs/mockups/Sensei/lib/sessions-zen.jsx → SessionsDigestZen
import type { HourActivity, SessionRow } from '$lib/types.js';

/** The chart chips (pulse is deliberately NOT here — it is a mini-cycler-only
 *  mode, see MINI_MODES). `tokens` is the per-day token-volume bars. */
export type ChartVariant = 'trend' | 'stream' | 'bands' | 'tokens';
export const CHART_VARIANTS: ChartVariant[] = ['trend', 'stream', 'bands', 'tokens'];

/** The collapsed-header mini-cycler modes. `numbers` is the resting stat trio;
 *  `pulse` lives ONLY here — it is never one of the CHART_VARIANTS chips. */
export type MiniMode = 'numbers' | 'trend' | 'stream' | 'bands' | 'pulse';
export const MINI_MODES: MiniMode[] = ['numbers', 'trend', 'stream', 'bands', 'pulse'];

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

/** The median duration as the totals show it: "—" only when no session has a
 *  duration, "<1m" for short ones (which round to 0). */
export function medianLabel(mins: number | null): string {
  if (mins == null) return '—';
  return mins < 1 ? '<1m' : compactDuration(mins);
}

/** A session's duration as every screen shows it: "12m", "1h 04m", "2h",
 *  "2d 3h"; "—" when it has none. */
export function compactDuration(mins: number | null): string {
  if (mins == null) return '—';
  if (mins < 60) return `${mins}m`;
  const h = Math.floor(mins / 60);
  if (h >= 24) {
    const d = Math.floor(h / 24);
    const rest = h % 24;
    return rest === 0 ? `${d}d` : `${d}d ${rest}h`;
  }
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

/**
 * Short axis label for a day key: "Today" / "Yest" / "mm/dd".
 *
 * Older days read as a real date rather than a "−N" offset. A negative number
 * makes the reader do arithmetic against an implicit today just to place a bar,
 * and it can't be matched against anything outside the chart — a commit, a
 * calendar, another screen. The date can. "Today"/"Yest" stay because they are
 * the two anchors people actually think in, and they carry no arithmetic.
 */
export function shortDayLabel(key: string, now: Date = new Date()): string {
  if (!key) return '';
  const [y, m, d] = key.split('-').map(Number);
  const then = new Date(y, (m ?? 1) - 1, d ?? 1);
  const days = calendarDaysAgo(then, now);
  if (days <= 0) return 'Today';
  if (days === 1) return 'Yest';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(m ?? 1)}/${pad(d ?? 1)}`;
}

/** A wire row plus every derived field the digest renders. */
export interface EnrichedSession {
  id: string;
  workspace: string;
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
  /** Folder role for multi-repo workspaces (`web` / `backend` / `docs`); null
   *  when the endpoint doesn't carry it (single-folder workspaces, or the
   *  observatory multi-workspace view). Drives the row's folder-role chip. */
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
      workspace: (s.workspace ?? '').trim(),
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
  workspaces: number;
  good: number;
  bad: number;
  ugly: number;
  neutral: number;
  /** Median duration in whole minutes; 0 is under a minute, null is no
   *  session with a duration. */
  medianMins: number | null;
  /** Total tokens over sessions carrying usage, and the per-session median.
   *  Null when NO visible session carried tokens (honest — not a 0 stat). */
  totalTokens: number | null;
  medianTokens: number | null;
}

export function computeTotals(sessions: EnrichedSession[]): DigestTotals {
  const tone = (t: QualityTone) => sessions.filter((s) => s.quality === t).length;
  const workspaces = new Set(sessions.map((s) => s.workspace).filter(Boolean)).size;
  const durations = sessions.map((s) => s.mins).filter((m): m is number => m != null);
  const tokenVals = sessions.map((s) => s.tokens).filter((t): t is number => t != null);
  return {
    count: sessions.length,
    workspaces,
    good: tone('good'),
    bad: tone('bad'),
    ugly: tone('ugly'),
    neutral: tone('neutral'),
    medianMins: durations.length ? Math.round(median(durations)) : null,
    totalTokens: tokenVals.length ? tokenVals.reduce((a, t) => a + t, 0) : null,
    medianTokens: tokenVals.length ? Math.round(median(tokenVals)) : null,
  };
}

/** Deep-link target for a row → Instruments Replay tab, session preselected.
 *  Session-id resolves via GET /api/sessions/{id} (shipped Slot 1). */
export function replayHref(sessionId: string): string {
  return `/instruments?tab=replay&session=${encodeURIComponent(sessionId)}`;
}

/** The words of the Sessions screen — Observatory-wide (`workspace` null) or one
 *  workspace's window. Everything else about the two is the same screen. */
export interface SessionsCopy {
  kanji: string;
  eyebrow: string;
  title: string;
  sub: string;
  emptyDescription: string;
  /** The totals line names how many workspaces the sessions span. */
  acrossWorkspaces: boolean;
}

export function sessionsCopy(workspace: string | null): SessionsCopy {
  if (workspace == null) {
    return {
      kanji: '録',
      eyebrow: 'Observatory · Sessions',
      title: 'The shape of your week.',
      sub: 'A retrospective. Then one quiet chart — drawn the way you want to read it.',
      emptyDescription:
        'Start a session with your assistant, or widen the range. Each session becomes a moment of learning.',
      acrossWorkspaces: true,
    };
  }
  return {
    kanji: '刻',
    eyebrow: `${workspace} · sessions`,
    title: 'The shape of this workspace.',
    sub: 'A retrospective — this workspace only. Then one quiet chart, drawn the way you read it.',
    emptyDescription:
      'Start a session with your assistant on this workspace, or widen the range. Each session becomes a moment of learning.',
    acrossWorkspaces: false,
  };
}


/** The 7-day rolling mean FTR at each day — over the scored days in the window,
 *  and only on a day that is itself scored (null otherwise). */
export function rollingFtr(days: { ftr: number | null }[]): (number | null)[] {
  return days.map((d, i) => {
    if (d.ftr == null) return null;
    const scored = days.slice(Math.max(0, i - 6), i + 1).filter((x) => x.ftr != null);
    return scored.reduce((a, x) => a + (x.ftr ?? 0), 0) / scored.length;
  });
}


/** The range a Sessions page opens on: the narrowest window that holds the newest
 *  session — 7 days when it ran within the week, 30 within the month, else 90
 *  (also when nothing ran). A quiet workspace no longer opens on an empty week. */
export function recencyRange(startedAts: string[], now: Date = new Date()): SessionRange {
  if (startedAts.length === 0) return '90d';
  const newest = Math.min(...startedAts.map((iso) => calendarDaysAgo(new Date(iso), now)));
  return newest < rangeDays('7d') ? '7d' : newest < rangeDays('30d') ? '30d' : '90d';
}

/** Every how-many days a day axis is labelled: each day up to two weeks, then
 *  about eight labels across. */
export function labelStride(days: number): number {
  return days > 14 ? Math.ceil(days / 8) : 1;
}

/** One local day of activity: minutes the agent worked and minutes spent on you. */
export interface ActivityDay {
  day: string;
  label: string;
  agentMins: number;
  userMins: number;
}

/** Each session's active time, filed by the hour it happened, summed per local day of the
 *  axis — the activity hotspots. A session running past midnight counts on both days;
 *  hours outside the axis are left out. Sessions not yet analysed (`activeByHour` null)
 *  are counted in `unmeasured`, never drawn as an empty day. */
export function activityByDay(
  rows: Pick<SessionRow, 'activeByHour'>[],
  days: string[],
  now: Date = new Date(),
): { days: ActivityDay[]; unmeasured: number } {
  const byDay = new Map(days.map((day) => [day, { agentMs: 0, userMs: 0 }]));
  let unmeasured = 0;
  for (const r of rows) {
    if (!r.activeByHour) {
      unmeasured += 1;
      continue;
    }
    for (const [hour, agentMs, userMs] of r.activeByHour) {
      const slot = byDay.get(dayKey(new Date(hour * 3_600_000).toISOString()));
      if (!slot) continue;
      slot.agentMs += agentMs;
      slot.userMs += userMs;
    }
  }
  return {
    days: days.map((day) => {
      const { agentMs, userMs } = byDay.get(day)!;
      return { day, label: shortDayLabel(day, now), agentMins: Math.round(agentMs / 60_000), userMins: Math.round(userMs / 60_000) };
    }),
    unmeasured,
  };
}

/** One local day of turns: how many, how many a correction followed, the first-try
 *  rate, and the tokens spent (null where none were captured — unknown, not 0). */
export interface TurnDay {
  day: string;
  label: string;
  turns: number;
  corrections: number;
  /** Turns not followed by a correction ÷ turns; null on a day with no turns. */
  ftr: number | null;
  /** The day's dot: corrected turns → bad, all first-try → good, none → neutral. */
  tone: QualityTone;
  tokensIn: number | null;
  tokensOut: number | null;
  tokens: number | null;
}

/** The hours summed per local day of the axis, so a session that ran for days counts
 *  on each of them (Trend, Stream and Tokens; user, 2026-10-29). */
export function turnsByDay(hours: HourActivity[], days: string[], now: Date = new Date()): TurnDay[] {
  const byDay = new Map(days.map((d) => [d, { turns: 0, corrections: 0, tin: null as number | null, tout: null as number | null }]));
  for (const [hour, turns, corrections, tin, tout] of hours) {
    const slot = byDay.get(dayKey(new Date(hour * 3_600_000).toISOString()));
    if (!slot) continue;
    slot.turns += turns;
    slot.corrections += corrections;
    if (tin !== null) slot.tin = (slot.tin ?? 0) + tin;
    if (tout !== null) slot.tout = (slot.tout ?? 0) + tout;
  }
  return days.map((day) => {
    const { turns, corrections, tin, tout } = byDay.get(day)!;
    const ftr = turns > 0 ? (turns - corrections) / turns : null;
    const tone: QualityTone = turns === 0 ? 'neutral' : corrections > 0 ? 'bad' : 'good';
    const tokens = tin === null && tout === null ? null : (tin ?? 0) + (tout ?? 0);
    return { day, label: shortDayLabel(day, now), turns, corrections, ftr, tone, tokensIn: tin, tokensOut: tout, tokens };
  });
}

/** A session active on more than one local day of the axis, and those days, in order. */
export interface SessionSpan {
  id: string;
  days: string[];
}

/** The sessions that ran over several days of the axis — one piece of work across
 *  several bars. From the same hours as the bars; a one-day session has no span. */
export function sessionSpans(rows: Pick<SessionRow, 'id' | 'activeByHour'>[], days: string[]): SessionSpan[] {
  const order = new Map(days.map((d, i) => [d, i]));
  const spans: SessionSpan[] = [];
  for (const r of rows) {
    if (!r.activeByHour) continue;
    const on = new Set<string>();
    for (const [hour] of r.activeByHour) {
      const day = dayKey(new Date(hour * 3_600_000).toISOString());
      if (order.has(day)) on.add(day);
    }
    if (on.size > 1) spans.push({ id: r.id, days: [...on].sort((a, b) => order.get(a)! - order.get(b)!) });
  }
  return spans.sort((a, b) => order.get(a.days[0])! - order.get(b.days[0])!);
}

/** Says how many sessions the bands could not place, or null when it placed them all. */
export function unmeasuredLine(n: number): string | null {
  if (n <= 0) return null;
  return `${n} ${n === 1 ? 'session' : 'sessions'} not analysed yet — not drawn`;
}

/** One stacked segment of a bands bar. */
export interface BandSegment {
  tone: 'agent' | 'user';
  y: number;
  h: number;
}

/** The bands chart laid out: a vertical bar per day, oldest left, the agent's minutes
 *  stacked under yours; date labels at `labelStride`, the newest day always labelled. */
export interface BandsView {
  plot: { left: number; right: number; top: number; bottom: number };
  bars: { day: string; label: string; x: number; w: number; segments: BandSegment[]; agentMins: number; userMins: number; mins: number }[];
  ticks: { x: number; label: string }[];
  yTicks: { y: number; label: string }[];
  maxMins: number;
  /** Multi-day sessions: a line over their first to last day, a dot on each active day. */
  links: { id: string; x1: number; x2: number; y: number; dots: number[] }[];
}

/** Height of one lane of multi-day links above the bars. */
const LINK_LANE = 8;

/** Fill per segment: the agent's time and yours. */
export const BAND_FILL: Record<BandSegment['tone'], string> = {
  agent: 'var(--info)',
  user: 'var(--accent)',
};

export function bandsView(days: ActivityDay[], width: number, height: number, spans: SessionSpan[] = []): BandsView {
  // Spans that overlap take separate lanes: each lane ends before the next span starts.
  const index = new Map(days.map((d, i) => [d.day, i]));
  const laneEnds: number[] = [];
  const laned = spans.map((span) => {
    const first = index.get(span.days[0])!;
    const last = index.get(span.days[span.days.length - 1])!;
    let lane = laneEnds.findIndex((end) => end < first);
    if (lane === -1) lane = laneEnds.length;
    laneEnds[lane] = last;
    return { span, lane };
  });
  const plot = { left: 44, right: width - 12, top: 12 + laneEnds.length * LINK_LANE, bottom: height - 26 };
  const n = Math.max(days.length, 1);
  const slot = (plot.right - plot.left) / n;
  const w = Math.max(1, slot * 0.7);
  // An hour at least, rounded up to half hours under four hours, whole hours above.
  const busiest = Math.max(60, ...days.map((d) => d.agentMins + d.userMins));
  const step = busiest <= 240 ? 30 : 60;
  const maxMins = Math.ceil(busiest / step) * step;
  const scale = (plot.bottom - plot.top) / maxMins;
  const bars = days.map((d, i) => {
    const x = plot.left + i * slot + (slot - w) / 2;
    let base = plot.bottom;
    const segments: BandSegment[] = [];
    for (const [tone, mins] of [['agent', d.agentMins], ['user', d.userMins]] as const) {
      if (mins <= 0) continue;
      const h = mins * scale;
      base -= h;
      segments.push({ tone, y: base, h });
    }
    return { day: d.day, label: d.label, x, w, segments, agentMins: d.agentMins, userMins: d.userMins, mins: d.agentMins + d.userMins };
  });
  const stride = labelStride(days.length);
  const last = days.length - 1;
  // Counted back from the newest day, so "Today" is always labelled and the
  // labels keep an even spacing behind it.
  const ticks = bars
    .filter((_, i) => (last - i) % stride === 0)
    .map((b) => ({ x: b.x + b.w / 2, label: b.label }));
  const yTicks = [0, maxMins / 2, maxMins].map((m) => ({ y: plot.bottom - m * scale, label: compactDuration(Math.round(m)) }));
  const centre = (day: string) => {
    const b = bars[index.get(day)!];
    return b.x + b.w / 2;
  };
  const links = laned.map(({ span, lane }) => ({
    id: span.id,
    x1: centre(span.days[0]),
    x2: centre(span.days[span.days.length - 1]),
    y: 8 + lane * LINK_LANE,
    dots: span.days.map(centre),
  }));
  return { plot, bars, ticks, yTicks, maxMins, links };
}
