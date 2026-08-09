// Project window · Overview (Slot 4) — pure view derivations.
//
// The daemon owns the numbers and every user-facing recommendation string
// (headline/body arrive resolved through insight-copy, with a static
// fallback). This module only projects the wire payload into the display
// contracts the pane renders: the FTR chip, the hero (top-rec vs all-quiet),
// the three stat blocks, the multi-repo chips, and the recent-session rows.
// No component carries logic — it all lives here so it can be unit-tested in
// isolation.
import type {
  ProjectOverviewHeader,
  ProjectOverviewTopRec,
  ProjectOverviewStats,
  ProjectOverviewSession,
} from '$lib/types.js';
import { ftrPct } from '$lib/ftr.js';

// ── Header FTR chip ─────────────────────────────────────────────────────────

export interface FtrDisplay {
  /** 14-day FTR as a whole-number percentage (0–100), or null when the project
   *  has no analyzed sessions in the window — the chip renders "—", never 0%. */
  pct: number | null;
  /** Text-color utility — warning when the project is flagged, else ink. */
  toneClass: string;
}

/** Project the header FTR chip. Reads `project.ftr` (the same store-backed FTR
 *  number — project_metrics, metric='ftr' — the projects-index card uses) so the
 *  two numbers never disagree. `pct` is null (render "—") when there's no FTR
 *  data — never a fabricated 0%. Warn tone is driven by `project.warn`, not the rate. */
export function ftrDisplay(project: Pick<ProjectOverviewHeader, 'ftr' | 'warn'>): FtrDisplay {
  return {
    pct: ftrPct(project.ftr),
    toneClass: project.warn ? 'text-warning' : 'text-ink',
  };
}

/** The header eyebrow — "Project · {client or internal}". */
export function projectEyebrow(project: Pick<ProjectOverviewHeader, 'client'>): string {
  return `Project · ${project.client?.trim() || 'internal'}`;
}

/** The "{n} repos" chip label, shown only when the project spans >1 folder.
 *  Null for a single-folder project — the caller renders that folder plainly. */
export function repoChipLabel(folders: ProjectOverviewHeader['folders']): string | null {
  return folders.length > 1 ? `${folders.length} repos` : null;
}

// ── Hero card ───────────────────────────────────────────────────────────────

export interface HeroContent {
  kanji: string;
  eyebrow: string;
  headline: string;
  body: string;
  /** "send to {acp}" — only when a rec AND a non-null defaultAcp both exist. */
  action: string | null;
  /** Evidence session ids joined with " · "; null when there are none. */
  meta: string | null;
  /** True when there is no pending recommendation — drives the still glyph. */
  quiet: boolean;
}

const QUIET_HEADLINE = 'All quiet — no urgent recommendations.';
const QUIET_BODY = 'Sensei is observing. The next correction or pattern will surface here.';

/** Build the hero from the top recommendation. When none is pending, render
 *  the all-quiet state (静, listening copy, no action) — never a "getting
 *  started" empty screen. Headline/body are the insight-copy strings the
 *  daemon already resolved; the fallbacks here match the mockup verbatim and
 *  cover the daemon-hiccup path. The send action appears only when a rec
 *  carries a non-null defaultAcp. */
export function heroContent(rec: ProjectOverviewTopRec | null): HeroContent {
  const eyebrow = 'This project · sensei speaks';
  if (!rec) {
    return { kanji: '静', eyebrow, headline: QUIET_HEADLINE, body: QUIET_BODY, action: null, meta: null, quiet: true };
  }
  const acp = rec.defaultAcp?.trim();
  return {
    kanji: '聴',
    eyebrow,
    headline: rec.title?.trim() || 'A recommendation is ready.',
    body: rec.why?.trim() || QUIET_BODY,
    action: acp ? `send to ${acp}` : null,
    meta: rec.evidence.length > 0 ? rec.evidence.join(' · ') : null,
    quiet: false,
  };
}

// ── Stat blocks ─────────────────────────────────────────────────────────────

export interface StatBlock {
  key: string;
  label: string;
  value: number;
  sub: string;
  /** Value tone — warning only when the number should draw the eye. */
  toneClass: string;
}

/** The three stat blocks: sessions / memories / doc-drift, each with the
 *  sub-line that carries the counts backing the headline number. Doc-drift
 *  takes the warning tone only when the open count is > 0 (spec wrong-gate:
 *  never warn on zero). */
export function statBlocks(stats: ProjectOverviewStats): StatBlock[] {
  const { memories: m, docDrift: d } = stats;
  return [
    {
      key: 'sessions',
      label: 'Sessions · 7d',
      value: stats.sessions7d,
      sub: `${stats.sessions7dCorrected} corrected`,
      toneClass: 'text-ink',
    },
    {
      key: 'memories',
      label: 'Memories',
      value: m.total,
      sub: `${m.readyToShare} to share · ${m.toMerge} to merge`,
      toneClass: 'text-ink',
    },
    {
      key: 'docDrift',
      label: 'Doc drift',
      value: d.open,
      sub: `of ${d.referencedDocs} referenced docs`,
      toneClass: d.open > 0 ? 'text-warning' : 'text-ink',
    },
  ];
}

// ── Recent-session rows ─────────────────────────────────────────────────────

export interface SessionRow {
  id: string;
  title: string;
  /** "{shortId} · {duration} · {corrections|first-try right}". */
  meta: string;
  /** Relative time, e.g. "2h" / "3d". */
  time: string;
  ftr: boolean;
  /** Time-column tone — success green on a first-try session, else muted. */
  timeToneClass: string;
  /** Folder role for the row's repo; null → no chip (single-repo/unassigned). */
  role: string | null;
}

/** First 8 chars of a UUID — a readable id for the mono sub-line without the
 *  full 36-char string dominating the row. */
function shortId(id: string): string {
  return id.length > 8 ? id.slice(0, 8) : id;
}

/** Non-blank row title. Hook-derived sessions (#31) carry no title, so fall
 *  back to a generic word rather than render an empty middle column. */
function sessionTitle(title: string): string {
  return title.trim() || 'session';
}

/** "3 corrections" vs "first-try right" — matches the mockup's phrasing. */
function correctionsPhrase(corrections: number): string {
  return corrections > 0 ? `${corrections} corrections` : 'first-try right';
}

/** Elapsed wall-clock between start and completion, formatted m / h m / d h.
 *  Null when the session has no completion or the timestamps don't parse. */
export function sessionDuration(startedAt: string, completedAt: string | null): string | null {
  if (!completedAt) return null;
  const start = Date.parse(startedAt);
  const end = Date.parse(completedAt);
  if (Number.isNaN(start) || Number.isNaN(end) || end <= start) return null;
  const mins = Math.round((end - start) / 60_000);
  if (mins < 60) return `${mins}m`;
  const hoursTotal = Math.floor(mins / 60);
  if (hoursTotal < 24) {
    const m = mins % 60;
    return m === 0 ? `${hoursTotal}h` : `${hoursTotal}h ${m}m`;
  }
  const d = Math.floor(hoursTotal / 24);
  const h = hoursTotal % 24;
  return h === 0 ? `${d}d` : `${d}d ${h}h`;
}

/** Terse relative-time label from an ISO timestamp — m / h / d / w / mo / y.
 *  `now` is injectable so the label is deterministic under test. Empty string
 *  for a missing/unparseable timestamp so the row's time column collapses. */
export function relativeTime(iso: string | null, now: number = Date.now()): string {
  if (!iso) return '';
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return '';
  const diff = now - then;
  if (diff < 60_000) return 'just now';
  if (diff < 3_600_000) return `${Math.round(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.round(diff / 3_600_000)}h`;
  if (diff < 7 * 86_400_000) return `${Math.round(diff / 86_400_000)}d`;
  if (diff < 30 * 86_400_000) return `${Math.round(diff / (7 * 86_400_000))}w`;
  if (diff < 365 * 86_400_000) return `${Math.round(diff / (30 * 86_400_000))}mo`;
  return `${Math.round(diff / (365 * 86_400_000))}y`;
}

/** Shape the recent-session rows: a non-blank title, a mono sub-line carrying
 *  id + duration + correction phrase, a relative time from completion (or from
 *  start when still running), the FTR flag + its tone, and the folder-role
 *  chip (null passes straight through so the row renders no chip). */
export function sessionRows(sessions: ProjectOverviewSession[], now: number = Date.now()): SessionRow[] {
  return sessions.map((s) => {
    const parts = [shortId(s.id), sessionDuration(s.startedAt, s.completedAt), correctionsPhrase(s.corrections)]
      .filter((p): p is string => Boolean(p));
    return {
      id: s.id,
      title: sessionTitle(s.title),
      meta: parts.join(' · '),
      time: relativeTime(s.completedAt ?? s.startedAt, now),
      ftr: s.ftr,
      timeToneClass: s.ftr ? 'text-success' : 'text-ink-mute',
      role: s.role,
    };
  });
}
