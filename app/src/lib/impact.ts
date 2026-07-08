// Measured-verdict domain logic — shared by the observatory-wide Impact
// screen and the project-scoped Impact tab. Both render the same primitive: a
// measured recommendation after the analyzer's MeasureVerdicts pass records a
// verdict. Pure functions + types only; no runes.

/** One entry in the MOE reasoning panel — a model plus the role it played and
 *  its per-model note. Written by `verdicts::synthesize_reasoning`. */
export interface ReasoningModel {
  name: string;
  /** proposer / challenger / synthesizer / reviewer, deterministic by the
   *  model's index in the panel. */
  role: string;
  note: string;
}

/** Rich MOE reasoning JSON attached to a measured recommendation. All fields
 *  are optional on the wire so a legacy `{conclusion}`-only trace still
 *  renders — the UI degrades gracefully to whatever it has. */
export interface ImpactReasoning {
  headline: string | null;
  body: string | null;
  /** "3 positive · 0 neutral · 0 negative" summary line. Null for legacy
   *  traces that didn't compose a per-panel breakdown. */
  consensus: string | null;
  models: ReasoningModel[];
  /** Populated only when the verdict is negative — an alternate approach the
   *  reader can try before rolling back entirely. */
  suggestedRevision: string | null;
}

/** Measured-verdict enum. `pending` covers accepted recs whose measurement
 *  window hasn't closed yet. */
export type ImpactVerdict = 'pending' | 'positive' | 'neutral' | 'negative';

/** Semantic tone a verdict maps to. `ink` = no strong signal (neutral /
 *  pending) — rendered in muted ink, never a status colour. */
export type VerdictTone = 'success' | 'warning' | 'ink';

/** One measured recommendation row. `projectId`/`projectName` are always
 *  present so the observatory-wide list can label cross-project rows; the
 *  project-scoped tab fills them from its own project and hides the column. */
export interface ImpactRow {
  id: string;
  projectId: string;
  projectName: string;
  title: string;
  status: string;
  /** Canonical action verb (`create_agent`, `enrich_memory`, …) or null when
   *  the daemon didn't classify one. */
  actionType: string | null;
  verdict: ImpactVerdict;
  baselineFtr: number | null;
  currentFtr: number | null;
  ftrDelta: number | null;
  reasoning: ImpactReasoning | null;
}

export interface ImpactBuckets {
  positive: ImpactRow[];
  neutral: ImpactRow[];
  negative: ImpactRow[];
  pending: ImpactRow[];
}

export interface VerdictMeta {
  glyph: string;
  tone: VerdictTone;
  label: string;
}

/** Verdict → glyph + tone + label. The lookup IS the rendering rule, so it
 *  lives with the domain rather than in a component. */
export const VERDICT_META: Record<ImpactVerdict, VerdictMeta> = {
  positive: { glyph: '好', tone: 'success', label: 'positive impact' },
  neutral:  { glyph: '並', tone: 'ink',     label: 'no measurable effect' },
  negative: { glyph: '悪', tone: 'warning', label: 'negative impact' },
  pending:  { glyph: '待', tone: 'ink',     label: 'measurement pending' },
};

/** Pure: resolve a verdict's meta, defaulting unknown/absent values to
 *  `pending` so an unexpected wire value degrades to the muted state. */
export function verdictMeta(v: string | null | undefined): VerdictMeta {
  return VERDICT_META[(v ?? 'pending') as ImpactVerdict] ?? VERDICT_META.pending;
}

/** Pure: named-token utility class for a verdict tone. `ink` → muted ink (no
 *  status colour). */
export function verdictToneClass(
  tone: VerdictTone,
  kind: 'text' | 'bg' | 'border' = 'text',
): string {
  const prefix = kind === 'bg' ? 'bg' : kind === 'border' ? 'border' : 'text';
  if (tone === 'success') return `${prefix}-success`;
  if (tone === 'warning') return `${prefix}-warning`;
  return `${prefix}-ink-mute`;
}

/** Pure: FTR value → percent string, or an em dash when unknown. */
export function pct(v: number | null): string {
  if (v == null) return '—';
  return `${Math.round(v * 100)}%`;
}

/** Pure: percent-signed delta ("+14%" / "-3%" / "±0%" / "—") for a row card. */
export function formatDeltaPct(v: number | null): string {
  if (v == null) return '—';
  const p = Math.round(v * 100);
  if (p === 0) return '±0%';
  return p > 0 ? `+${p}%` : `${p}%`;
}

/** Pure: bucket measured rows by verdict tone.
 *  - positive/negative first (they carry the sharpest signal)
 *  - pending covers accepted recs whose measurement window hasn't closed
 *  Sorted by |ftrDelta| desc so the largest-effect rows lead each bucket. */
export function bucketImpact(rows: ImpactRow[]): ImpactBuckets {
  const buckets: ImpactBuckets = { positive: [], neutral: [], negative: [], pending: [] };
  for (const r of rows) {
    if (r.verdict === 'positive' || r.verdict === 'negative' || r.verdict === 'neutral' || r.verdict === 'pending') {
      buckets[r.verdict].push(r);
    }
  }
  const magnitude = (v: number | null) => (v == null ? -1 : Math.abs(v));
  for (const key of ['positive', 'negative', 'neutral', 'pending'] as const) {
    buckets[key].sort((a, b) => magnitude(b.ftrDelta) - magnitude(a.ftrDelta));
  }
  return buckets;
}
