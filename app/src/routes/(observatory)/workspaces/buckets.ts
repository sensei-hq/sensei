import { ftrBand, ftrBandView } from '$lib/ftr';
import { KIND_ICONS } from '$lib/glyph-vocabulary';
import type { GlyphMode } from '$lib/glyphs';
import type { WorkspaceListItem } from '$lib/types.js';
import { failureReason, type ApiResult } from '$lib/api.js';

/** One row in the segmented Workspaces list — a workspace enriched with the
 *  signals shown on its card (FTR%, 7-day sessions, warning dot) plus the
 *  list-endpoint aggregates the card/row render (repos/libs/last-session/
 *  vision). Counts are normalized to numbers by `baseWorkspace` so the pure
 *  components never have to reach for a fallback. */
export interface EnrichedWorkspace extends WorkspaceListItem {
  /** 14-day FTR, 0..1 — null when the workspace has no analyzed sessions in the
   *  window (honest no-data, rendered "—"; never a fabricated 0%). */
  ftr14d: number | null;
  sessions7d: number;
  /** True when a quality signal is red (open drift, or FTR7d below 60%).
   *  Drives the amber dot the mockup shows next to the workspace name. */
  warn: boolean;
  repos_count: number;
  libs_count: number;
  last_session_at: string | null;
  vision: string | null;
  /** Why the card's quality reads (FTR, signals) failed, or null (#288). The
   *  card then says "unavailable" — not a blank rate, not a made-up warning. */
  readError: string | null;
}

/** A workspace's card from the list payload alone, before any quality read. */
export function baseWorkspace(p: WorkspaceListItem): EnrichedWorkspace {
  return {
    ...p,
    ftr14d: null,
    sessions7d: p.sessions7d ?? 0,
    warn: false,
    repos_count: p.repos_count ?? 0,
    libs_count: p.libs_count ?? 0,
    last_session_at: p.last_session_at ?? null,
    vision: p.vision ?? null,
    readError: null,
  };
}

/** The card with its quality reads applied: the rate and the warning, or —
 *  when either read failed — the reason, and neither (#288). */
export function withQuality(
  base: EnrichedWorkspace,
  ftr: ApiResult<{ ftr14d: number | null }>,
  signals: ApiResult<{ open_drift_count: number; ftr_7d: number }>,
): EnrichedWorkspace {
  if (!ftr.ok) return { ...base, readError: failureReason(ftr.error) };
  if (!signals.ok) return { ...base, readError: failureReason(signals.error) };
  return { ...base, ftr14d: ftr.data.ftr14d, warn: isWarning(signals.data, base.sessions7d) };
}

/** The three mutually-exclusive buckets the screen surfaces. A workspace
 *  lands in exactly one, so the chip counts always sum to `all`. */
export interface Buckets {
  active:   EnrichedWorkspace[];
  dormant:  EnrichedWorkspace[];
  archived: EnrichedWorkspace[];
}

export type WorkspaceStatus = 'active' | 'dormant' | 'archived';

/** Pure: derive a workspace's status from real wire fields. `archived`
 *  (maturity enum) takes priority over activity so an archived workspace is
 *  never double-counted as active. A workspace is `active` when it has any
 *  session in the last 7 days, otherwise `dormant`. */
export function workspaceStatus(
  p: Pick<EnrichedWorkspace, 'sessions7d' | 'maturity'>,
): WorkspaceStatus {
  if (p.maturity === 'archived') return 'archived';
  return p.sessions7d > 0 ? 'active' : 'dormant';
}

/** Parse `last_session_at` to epoch ms for recency ordering. Null / unparseable
 *  timestamps sort LAST (a workspace that never ran a session is the least recent),
 *  so they use -Infinity. Pure. */
function recencyOf(p: EnrichedWorkspace): number {
  const t = p.last_session_at ? new Date(p.last_session_at).getTime() : NaN;
  return Number.isNaN(t) ? -Infinity : t;
}

/** Pure: split enriched workspaces into Active / Dormant / Archived and sort each
 *  by MOST-RECENT activity first (last_session_at desc), name as the tiebreak.
 *  Buckets are mutually exclusive (see `workspaceStatus`) so the union is the full
 *  list and the chip counts sum to `all`. Recency-first matches how you scan the
 *  list — what you touched last is what you reach for; FTR stays a per-card stat. */
export function bucketWorkspaces(workspaces: EnrichedWorkspace[]): Buckets {
  const active:   EnrichedWorkspace[] = [];
  const dormant:  EnrichedWorkspace[] = [];
  const archived: EnrichedWorkspace[] = [];
  for (const p of workspaces) {
    switch (workspaceStatus(p)) {
      case 'archived': archived.push(p); break;
      case 'active':   active.push(p);   break;
      default:         dormant.push(p);  break;
    }
  }
  const byRecency = (a: EnrichedWorkspace, b: EnrichedWorkspace) =>
    recencyOf(b) - recencyOf(a) || a.name.localeCompare(b.name);
  active.sort(byRecency);
  dormant.sort(byRecency);
  archived.sort(byRecency);
  return { active, dormant, archived };
}

/** Pure: a workspace is "warning" if it has open drift OR its 7-day FTR is
 *  below 0.6 — but only when it has recent activity. A dormant workspace has no
 *  FTR signal, and the wire's zero-FTR fallback (`ftr_7d: 0`) would otherwise
 *  light every dormant workspace amber, drowning the signal at scale. Guard on
 *  `sessions7d` so the dot only warns about workspaces actually being worked on. */
export function isWarning(
  signals: { open_drift_count: number; ftr_7d: number },
  sessions7d: number,
): boolean {
  if (sessions7d <= 0) return false;
  return signals.open_drift_count > 0 || ftrBand(signals.ftr_7d) === 'low';
}

/** The dot beside a workspace: the shared FTR band (lib/ftr), unless a live
 *  warning wins. */
export function workspaceDotView(ftr: number | null, warn: boolean): { dot: string; label: string } {
  if (warn) return { dot: 'bg-warning', label: 'needs attention' };
  const v = ftrBandView(ftr);
  return { dot: v.dot, label: v.label };
}

/** The default hero/list glyph — 場 (place) — when nothing else is known.
 *  Mirrors the daemon's `workspace_overview::DEFAULT_KANJI` fallback. */
export const DEFAULT_GLYPH = '場';

/** The card/row icon, resolved so a kanji glyph and an inferred image can
 *  never render together (the fallback OR-collapse is the wrong-gate). The
 *  `image` variant also carries `glyph` — the kanji to fall back to when the
 *  `<img>` fails to load (a broken image is worse than a glyph). */
export type WorkspaceIcon =
  | { kind: 'image'; src: string; glyph: string }
  | { kind: 'kanji'; glyph: string }
  | { kind: 'icon'; cls: string; glyph: string };

/** A workspace's mark in icon mode (#133) — the kind icon from the shared vocabulary. */
export const WORKSPACE_ICON = KIND_ICONS.workspace;

/** Pure: does a stored icon value already point somewhere the browser can load
 *  directly? An author/About-form icon may be an absolute URL or a data URI;
 *  a machine-inferred image is a repo-relative path that must go through the
 *  daemon's serve route instead. */
function isDirectUrl(value: string): boolean {
  return /^(?:https?:)?\/\//i.test(value) || value.startsWith('data:');
}

/** Pure: pick the workspace's icon. An `image` icon wins only when it carries a
 *  value — an absolute URL/data URI is used as-is, while a repo-relative path
 *  is served by the daemon at `{base}/api/workspaces/{id}/icon` (the app renders
 *  it with a kanji fallback on load error). Otherwise fall back to the kanji
 *  glyph on the icon, and finally to 場 (place). `base` is the daemon origin
 *  (e.g. `apiBase(port)`); it defaults to '' so tests read a bare route path. */
export function workspaceIcon(
  p: Pick<EnrichedWorkspace, 'id' | 'icon'>,
  base = '',
  mode: GlyphMode = 'kanji',
): WorkspaceIcon {
  const icon = p.icon;
  // An image the daemon says it cannot serve is no image: asking would 404.
  if (icon && icon.kind === 'image' && icon.value && icon.available !== false) {
    const src = isDirectUrl(icon.value)
      ? icon.value
      : `${base}/api/workspaces/${encodeURIComponent(p.id)}/icon`;
    return { kind: 'image', src, glyph: DEFAULT_GLYPH };
  }
  const glyph = icon && icon.kind === 'kanji' && icon.value ? icon.value : DEFAULT_GLYPH;
  // Every glyph follows the mode (#133): a chosen kanji too, kept as the fallback.
  if (mode === 'icon') return { kind: 'icon', cls: WORKSPACE_ICON, glyph };
  return { kind: 'kanji', glyph };
}

/** Pure: compact "last session" label from an ISO timestamp. `now` is
 *  injectable so the label is deterministic under test. Null / unparseable
 *  timestamps render as an em dash — a dormant workspace may simply have
 *  never run a session. Spans grow coarser with age: m → h → d → w → mo → y. */
export function lastSessionLabel(iso: string | null, now: number = Date.now()): string {
  if (!iso) return '—';
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '—';
  const diff = now - then;
  if (diff < 60_000)         return 'just now';
  if (diff < 3_600_000)      return `${Math.round(diff / 60_000)}m`;
  if (diff < 86_400_000)     return `${Math.round(diff / 3_600_000)}h`;
  if (diff < 7 * 86_400_000) return `${Math.round(diff / 86_400_000)}d`;
  if (diff < 30 * 86_400_000)  return `${Math.round(diff / (7 * 86_400_000))}w`;
  if (diff < 365 * 86_400_000) return `${Math.round(diff / (30 * 86_400_000))}mo`;
  return `${Math.round(diff / (365 * 86_400_000))}y`;
}
