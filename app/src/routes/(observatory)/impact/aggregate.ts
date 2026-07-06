/** One entry in the MOE reasoning panel — a model plus the role it played
 *  and its per-model note. Written by `verdicts::synthesize_reasoning`. */
export interface ReasoningModel {
  name: string;
  /** proposer / challenger / synthesizer / reviewer, deterministic by
   *  the model's index in the panel. */
  role: string;
  note: string;
}

/** Rich MOE reasoning JSON attached to a measured recommendation.
 *  All fields are optional on the wire so a legacy `{conclusion}`-only
 *  trace still renders — the UI degrades gracefully to whatever it has. */
export interface ImpactReasoning {
  headline: string | null;
  body: string | null;
  /** "3 positive · 0 neutral · 0 negative" summary line. Null for
   *  legacy traces that didn't compose a per-panel breakdown. */
  consensus: string | null;
  models: ReasoningModel[];
  /** Populated only when verdict is negative — an alternate approach
   *  the reader can try before rolling back entirely. */
  suggestedRevision: string | null;
}

/** One row in the observatory-wide Impact list — a measured recommendation
 *  after the analyzer's MeasureVerdicts pass has recorded a verdict. */
export interface ImpactRow {
  id: string;
  projectId: string;
  projectName: string;
  title: string;
  status: string;
  verdict: 'pending' | 'positive' | 'neutral' | 'negative';
  baselineFtr: number | null;
  currentFtr: number | null;
  ftrDelta: number | null;
  reasoning: ImpactReasoning | null;
}

export interface ImpactBuckets {
  positive: ImpactRow[];
  neutral:  ImpactRow[];
  negative: ImpactRow[];
  pending:  ImpactRow[];
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

/** Pure: percent-signed delta ("+14%" / "-3%" / "—") for the row card. */
export function formatDeltaPct(v: number | null): string {
  if (v == null) return '—';
  const pct = Math.round(v * 100);
  if (pct === 0)  return '±0%';
  return pct > 0 ? `+${pct}%` : `${pct}%`;
}
