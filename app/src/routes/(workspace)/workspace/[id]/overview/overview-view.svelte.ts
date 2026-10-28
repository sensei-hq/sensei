// Workspace window · Overview (Slot 4) — pure view derivations.
//
// The daemon owns the numbers and every user-facing recommendation string
// (headline/body arrive resolved through narration-cache, with a static
// fallback). This module only workspaces the wire payload into the display
// contracts the pane renders: the FTR chip, the hero (top-rec vs all-quiet),
// the three stat blocks, the multi-repo chips, and the recent-session rows.
// No component carries logic — it all lives here so it can be unit-tested in
// isolation.
import { correctionsView } from '$lib/ftr';
import type {
  WorkspaceOverviewHeader,
  WorkspaceOverviewTopRec,
  WorkspaceOverviewStats,
  WorkspaceOverviewSession,
  WorkspaceOverview,
  PatternEntry,
} from '$lib/types.js';
import { ftrPct } from '$lib/ftr.js';
import { failureReason, readOutcome, type ApiResult, type RecommendationPreview } from '$lib/api.js';

// ── Header FTR chip ─────────────────────────────────────────────────────────

export interface FtrDisplay {
  /** 14-day FTR as a whole-number percentage (0–100), or null when the workspace
   *  has no analyzed sessions in the window — the chip renders "—", never 0%. */
  pct: number | null;
  /** Text-color utility — warning when the workspace is flagged, else ink. */
  toneClass: string;
}

/** Workspace the header FTR chip. Reads `workspace.ftr` (the same store-backed FTR
 *  number — workspace_metrics, metric='ftr' — the workspaces-index card uses) so the
 *  two numbers never disagree. `pct` is null (render "—") when there's no FTR
 *  data — never a fabricated 0%. Warn tone is driven by `workspace.warn`, not the rate. */
export function ftrDisplay(workspace: Pick<WorkspaceOverviewHeader, 'ftr' | 'warn'>): FtrDisplay {
  return {
    pct: ftrPct(workspace.ftr),
    toneClass: workspace.warn ? 'text-warning' : 'text-ink',
  };
}

/** The header eyebrow — "Workspace · {client or internal}". */
export function workspaceEyebrow(workspace: Pick<WorkspaceOverviewHeader, 'client'>): string {
  return `Workspace · ${workspaceVisibilityLabel(workspace.client)}`;
}

/** The client name, or "internal" when a workspace has none. One source for the
 *  fallback so the overview eyebrow and the sidebar chip can't drift apart. */
export function workspaceVisibilityLabel(client: string | null | undefined): string {
  return client?.trim() || 'internal';
}

/** The "{n} repos" chip label, shown only when the workspace spans >1 folder.
 *  Null for a single-folder workspace — the caller renders that folder plainly. */
export function repoChipLabel(folders: WorkspaceOverviewHeader['folders']): string | null {
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
 *  started" empty screen. Headline/body are the narration-cache strings the
 *  daemon already resolved; the fallbacks here match the mockup verbatim and
 *  cover the daemon-hiccup path. The send action appears only when a rec
 *  carries a non-null defaultAcp. */
export function heroContent(rec: WorkspaceOverviewTopRec | null): HeroContent {
  const eyebrow = 'This workspace · sensei speaks';
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
export function statBlocks(stats: WorkspaceOverviewStats): StatBlock[] {
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
  /** "{shortId} · {duration} · {corrections}" — the shared corrections words. */
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
export function sessionRows(sessions: WorkspaceOverviewSession[], now: number = Date.now()): SessionRow[] {
  return sessions.map((s) => {
    const parts = [shortId(s.id), sessionDuration(s.startedAt, s.completedAt), correctionsView(s.corrections).label]
      .filter((p): p is string => Boolean(p));
    return {
      id: s.id,
      title: sessionTitle(s.title),
      meta: parts.join(' · '),
      time: relativeTime(s.completedAt ?? s.startedAt, now),
      ftr: s.ftr,
      timeToneClass: s.ftr ? 'text-success' : 'text-ink-soft',
      role: s.role,
    };
  });
}

/** One file where edits concentrated, as `GET /api/workspaces/{id}/hotspots` sends it. */
export interface Hotspot {
  folder: string;
  file_path: string;
  edit_count: number;
  correction_count: number;
}

export interface HotspotRow {
  path: string;
  repo: string;
  edits: number;
  corrections: number;
}

/** The five most-edited files (#85), each path relative to its repository —
 *  or whole, when the repository's name is not in it. */
export function hotspotRows(hotspots: Hotspot[], limit = 5): HotspotRow[] {
  return [...hotspots]
    .sort((a, b) => b.edit_count - a.edit_count || b.correction_count - a.correction_count)
    .slice(0, limit)
    .map((h) => {
      const marker = `/${h.folder}/`;
      const at = h.file_path.lastIndexOf(marker);
      return {
        path: at >= 0 ? h.file_path.slice(at + marker.length) : h.file_path,
        repo: h.folder,
        edits: h.edit_count,
        corrections: h.correction_count,
      };
    });
}

export interface PatternSnapshot {
  followed: number;
  anti: number;
  topFollowed: string[];
  topAnti: string[];
}

/** How many patterns are in use, and the three most-used of each kind (#85). */
export function patternSnapshot(p: {
  followed: Pick<PatternEntry, 'name' | 'instanceCount'>[];
  antiPatterns: Pick<PatternEntry, 'name' | 'instanceCount'>[];
}): PatternSnapshot {
  const top = (list: Pick<PatternEntry, 'name' | 'instanceCount'>[]) =>
    [...list]
      .sort((a, b) => (b.instanceCount ?? 0) - (a.instanceCount ?? 0))
      .slice(0, 3)
      .map((x) => x.name);
  return {
    followed: p.followed.length,
    anti: p.antiPatterns.length,
    topFollowed: top(p.followed),
    topAnti: top(p.antiPatterns),
  };
}

type Read<T> = ApiResult<T>;

/** The overview page's data: each section's payload, or why it could not be
 *  read. A failed read is never replaced by an all-quiet default (#85). */
export function overviewPageData(
  overview: Read<WorkspaceOverview>,
  hotspots: Read<{ hotspots: Hotspot[] }>,
  patterns: Read<{ followed: PatternEntry[]; antiPatterns: PatternEntry[] }>,
) {
  const o = readOutcome(overview);
  const h = readOutcome(hotspots);
  const p = readOutcome(patterns);
  return {
    overview: o.data,
    overviewError: o.error,
    hotspots: h.data?.hotspots ?? null,
    hotspotsError: h.error,
    patterns: p.data,
    patternsError: p.error,
  };
}

// ── Accept / reject the top recommendation ──────────────────────────────────

/** The writes a decision makes — injected so a spec can fail them. */
export interface DecideApi {
  tryPreviewRecommendation: (workspaceId: string, recId: string) => Promise<ApiResult<RecommendationPreview>>;
  tryMaterializeRecommendation: (
    workspaceId: string,
    recId: string,
    opts: { gov_scope?: string; enforcement?: string },
  ) => Promise<ApiResult<unknown>>;
  tryAcceptWorkspaceRecommendation: (workspaceId: string, recId: string) => Promise<ApiResult<unknown>>;
  tryRejectWorkspaceRecommendation: (workspaceId: string, recId: string) => Promise<ApiResult<unknown>>;
}

/**
 * Accept or reject a recommendation. Accept materializes the rec's artifact:
 *  • file-class (write_skill / create_agent) → a workspace FILE, written only
 *    after `confirmWrite` agrees (it is shown the target path); declining
 *    leaves the rec pending;
 *  • rule-class → a governance rule at workspace scope, recommended tier;
 *  • anything else → the plain accept (status flip + FTR measurement).
 *
 * `done` is whether the decision was recorded; `error` is why it was not. A
 * preview that cannot be read stops here (#288): falling through to the plain
 * accept would mark the rec accepted with nothing written.
 */
export async function decideRecommendation(
  api: DecideApi,
  workspaceId: string,
  recId: string,
  action: 'accept' | 'reject',
  confirmWrite: (message: string) => boolean,
): Promise<{ done: boolean; error: string | null }> {
  const settle = (r: ApiResult<unknown>) =>
    r.ok ? { done: true, error: null } : { done: false, error: failureReason(r.error) };
  if (action === 'reject') return settle(await api.tryRejectWorkspaceRecommendation(workspaceId, recId));

  const preview = await api.tryPreviewRecommendation(workspaceId, recId);
  if (!preview.ok) return { done: false, error: failureReason(preview.error) };
  const p = preview.data;
  if (p.materializable && p.consent_required) {
    const message = `Create a ${p.kind} at ${p.path}?\n\nsensei will write a new file into this repo (git-tracked, reversible).`;
    if (!confirmWrite(message)) return { done: false, error: null };
    return settle(await api.tryMaterializeRecommendation(workspaceId, recId, {}));
  }
  if (p.materializable) {
    return settle(
      await api.tryMaterializeRecommendation(workspaceId, recId, { gov_scope: 'workspace', enforcement: 'recommended' }),
    );
  }
  return settle(await api.tryAcceptWorkspaceRecommendation(workspaceId, recId));
}
