// Derivations for the composite-health radar — the "all metrics combined" view.
//
// The daemon already does the judging: `GET /api/projects/{id}/health` returns
// each included metric's 0-5 `rating` (its reading scored against that metric's
// rating_scale) plus the `weight` it carries in the composite score. This module
// only shapes those into radar rows + axes; it never re-derives a rating, so a
// spoke and the score can't disagree (spec invariant I5).
//
// Why ratings and not raw values: the raw scales are unplottable together.
// `module_quality` lives inside 0-0.005 while `throughput` is sessions/day —
// one shared radial axis would flatten every spoke but one. The 0-5 rating is
// the common scale the score itself already uses.

/** One metric's contribution, as the daemon reports it. */
export interface HealthComponent {
  name: string;
  rating: number;
  weight: number;
}

/** `GET /api/projects/{id}/health`. Absent when nothing was rated. */
export interface ProjectHealth {
  health_score: number;
  rated_metrics: number;
  components: Record<string, HealthComponent>;
}

/** A radar row: one metric's spoke. */
export interface RadarSpoke {
  metric: string;
  label: string;
  rating: number;
  weight: number;
}

/** Per-axis config handed to `RadarChart`. */
export interface RadarAxis {
  key: string;
  label: string;
  domain: [number, number];
  weight: number;
}

/** Ratings are counts of passed thresholds, so the scale is fixed at 0-5. */
export const RATING_DOMAIN: [number, number] = [0, 5];

/**
 * Order spokes deterministically: heaviest first, then by label.
 *
 * A radar's silhouette is a function of axis ORDER, so an unstable order would
 * redraw the same data as a different shape on every load and make the chart
 * useless for comparison. Weight-first also puts the metrics that actually move
 * the score adjacent, so a dent in the heavy arc reads as significant rather
 * than being scattered among low-weight spokes.
 */
export function toSpokes(components: Record<string, HealthComponent>): RadarSpoke[] {
  return Object.entries(components ?? {})
    .filter(([, c]) => c && Number.isFinite(c.rating))
    .map(([metric, c]) => ({
      metric,
      label: c.name || metric,
      rating: c.rating,
      weight: c.weight,
    }))
    .sort((a, b) => b.weight - a.weight || a.label.localeCompare(b.label));
}

/** Axis specs in the same order as {@link toSpokes}, pinned to the 0-5 domain. */
export function toAxes(spokes: RadarSpoke[]): RadarAxis[] {
  return spokes.map((s) => ({
    key: s.metric,
    label: s.label,
    domain: RATING_DOMAIN,
    weight: s.weight,
  }));
}

/**
 * Coarse band for the composite score, for the centre readout's tone.
 *
 * Thresholds mirror the rating bands the score is built from (each rating step
 * is 20 points), so a "good" here means the weighted mean sits at 4+/5.
 */
export function scoreTone(score: number): 'success' | 'warning' | 'danger' {
  if (score >= 80) return 'success';
  if (score >= 50) return 'warning';
  return 'danger';
}

/**
 * How much of the model backed a score, as a 0-1 share of total weight.
 *
 * Exposed so a caller can say "18 of 21 metrics" rather than implying the score
 * covers everything — a number over a third of the model is not the same claim
 * as one over all of it.
 */
export function weightCoverage(spokes: RadarSpoke[], totalModelWeight: number): number {
  if (totalModelWeight <= 0) return 0;
  const w = spokes.reduce((sum, s) => sum + (Number.isFinite(s.weight) ? s.weight : 0), 0);
  return Math.min(1, w / totalModelWeight);
}
