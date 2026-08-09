import type { ProjectListItem } from '$lib/types.js';

/** One row in the segmented Projects list — a project enriched with the
 *  signals shown on its card (FTR%, 7-day sessions, warning dot) plus the
 *  list-endpoint aggregates the card/row render (repos/libs/last-session/
 *  vision). Counts are normalized to numbers in `+page.ts` so the pure
 *  components never have to reach for a fallback. */
export interface EnrichedProject extends ProjectListItem {
  /** 14-day FTR, 0..1 — null when the project has no analyzed sessions in the
   *  window (honest no-data, rendered "—"; never a fabricated 0%). */
  ftr14d: number | null;
  sessions7d: number;
  /** True when a quality signal is red (open drift, or FTR7d below 60%).
   *  Drives the amber dot the mockup shows next to the project name. */
  warn: boolean;
  repos_count: number;
  libs_count: number;
  last_session_at: string | null;
  vision: string | null;
}

/** The three mutually-exclusive buckets the screen surfaces. A project
 *  lands in exactly one, so the chip counts always sum to `all`. */
export interface Buckets {
  active:   EnrichedProject[];
  dormant:  EnrichedProject[];
  archived: EnrichedProject[];
}

export type ProjectStatus = 'active' | 'dormant' | 'archived';

/** Pure: derive a project's status from real wire fields. `archived`
 *  (maturity enum) takes priority over activity so an archived project is
 *  never double-counted as active. A project is `active` when it has any
 *  session in the last 7 days, otherwise `dormant`. */
export function projectStatus(
  p: Pick<EnrichedProject, 'sessions7d' | 'maturity'>,
): ProjectStatus {
  if (p.maturity === 'archived') return 'archived';
  return p.sessions7d > 0 ? 'active' : 'dormant';
}

/** Parse `last_session_at` to epoch ms for recency ordering. Null / unparseable
 *  timestamps sort LAST (a project that never ran a session is the least recent),
 *  so they use -Infinity. Pure. */
function recencyOf(p: EnrichedProject): number {
  const t = p.last_session_at ? new Date(p.last_session_at).getTime() : NaN;
  return Number.isNaN(t) ? -Infinity : t;
}

/** Pure: split enriched projects into Active / Dormant / Archived and sort each
 *  by MOST-RECENT activity first (last_session_at desc), name as the tiebreak.
 *  Buckets are mutually exclusive (see `projectStatus`) so the union is the full
 *  list and the chip counts sum to `all`. Recency-first matches how you scan the
 *  list — what you touched last is what you reach for; FTR stays a per-card stat. */
export function bucketProjects(projects: EnrichedProject[]): Buckets {
  const active:   EnrichedProject[] = [];
  const dormant:  EnrichedProject[] = [];
  const archived: EnrichedProject[] = [];
  for (const p of projects) {
    switch (projectStatus(p)) {
      case 'archived': archived.push(p); break;
      case 'active':   active.push(p);   break;
      default:         dormant.push(p);  break;
    }
  }
  const byRecency = (a: EnrichedProject, b: EnrichedProject) =>
    recencyOf(b) - recencyOf(a) || a.name.localeCompare(b.name);
  active.sort(byRecency);
  dormant.sort(byRecency);
  archived.sort(byRecency);
  return { active, dormant, archived };
}

/** Pure: a project is "warning" if it has open drift OR its 7-day FTR is
 *  below 0.6 — but only when it has recent activity. A dormant project has no
 *  FTR signal, and the wire's zero-FTR fallback (`ftr_7d: 0`) would otherwise
 *  light every dormant project amber, drowning the signal at scale. Guard on
 *  `sessions7d` so the dot only warns about projects actually being worked on. */
export function isWarning(
  signals: { open_drift_count: number; ftr_7d: number },
  sessions7d: number,
): boolean {
  if (sessions7d <= 0) return false;
  return signals.open_drift_count > 0 || signals.ftr_7d < 0.6;
}

/** The default hero/list glyph — 場 (place) — when nothing else is known.
 *  Mirrors the daemon's `project_overview::DEFAULT_KANJI` fallback. */
export const DEFAULT_GLYPH = '場';

/** The card/row icon, resolved so a kanji glyph and an inferred image can
 *  never render together (the fallback OR-collapse is the wrong-gate). The
 *  `image` variant also carries `glyph` — the kanji to fall back to when the
 *  `<img>` fails to load (a broken image is worse than a glyph). */
export type ProjectIcon =
  | { kind: 'image'; src: string; glyph: string }
  | { kind: 'kanji'; glyph: string };

/** Pure: does a stored icon value already point somewhere the browser can load
 *  directly? An author/About-form icon may be an absolute URL or a data URI;
 *  a machine-inferred image is a repo-relative path that must go through the
 *  daemon's serve route instead. */
function isDirectUrl(value: string): boolean {
  return /^(?:https?:)?\/\//i.test(value) || value.startsWith('data:');
}

/** Pure: pick the project's icon. An `image` icon wins only when it carries a
 *  value — an absolute URL/data URI is used as-is, while a repo-relative path
 *  is served by the daemon at `{base}/api/projects/{id}/icon` (the app renders
 *  it with a kanji fallback on load error). Otherwise fall back to the kanji
 *  glyph on the icon, and finally to 場 (place). `base` is the daemon origin
 *  (e.g. `apiBase(port)`); it defaults to '' so tests read a bare route path. */
export function projectIcon(p: Pick<EnrichedProject, 'id' | 'icon'>, base = ''): ProjectIcon {
  const icon = p.icon;
  if (icon && icon.kind === 'image' && icon.value) {
    const src = isDirectUrl(icon.value)
      ? icon.value
      : `${base}/api/projects/${encodeURIComponent(p.id)}/icon`;
    return { kind: 'image', src, glyph: DEFAULT_GLYPH };
  }
  if (icon && icon.kind === 'kanji' && icon.value) return { kind: 'kanji', glyph: icon.value };
  return { kind: 'kanji', glyph: DEFAULT_GLYPH };
}

/** Pure: compact "last session" label from an ISO timestamp. `now` is
 *  injectable so the label is deterministic under test. Null / unparseable
 *  timestamps render as an em dash — a dormant project may simply have
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
