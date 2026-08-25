// Plotting two correlated metrics on one chart.
//
// The problem this solves is SCALE. A correlated pair almost never shares units:
// `ftr` lives in [0,1] while `tokens_per_day` runs to millions, so drawing both
// against one axis renders the smaller series as a flat line on the floor.
//
// The usual fix — a second Y axis — is rejected here. Independent axes can be
// slid and stretched until any two series appear to move together, which is
// exactly the illusion this screen must not create when its whole purpose is
// showing a real relationship. Instead both series are min-max normalised to
// [0,1] over the plotted window and the axis is labelled as normalised. That
// keeps SHAPE (which is what a rank correlation is about) and drops magnitude
// (which was never comparable).
//
// Pure: no fetching, no formatting decisions beyond the numbers.

/** One metric's normalised series, ready to draw. */
export interface NormalizedSeries {
    key: string;
    /** Values in [0,1], `null` preserved so a gap stays a gap. */
    points: (number | null)[];
    /** The real min/max, so the UI can label what 0 and 1 meant. */
    min: number;
    max: number;
}

/** A correlation as the daemon reports it. */
export interface MetricCorrelation {
    a: string;
    b: string;
    rho: number;
    n: number;
}

/**
 * Min-max normalise to [0,1], preserving nulls.
 *
 * A FLAT series (every value equal) maps to 0.5, not 0 — dividing by a zero
 * range would give NaN, and pinning it to the floor would read as "this metric
 * collapsed" when it actually never moved.
 *
 * `null` when there is nothing to draw (no defined values): the caller renders an
 * empty state rather than a line at an arbitrary height.
 */
export function normalize(key: string, points: (number | null)[]): NormalizedSeries | null {
    const defined = points.filter((p): p is number => p != null && Number.isFinite(p));
    if (defined.length === 0) return null;
    const min = Math.min(...defined);
    const max = Math.max(...defined);
    const range = max - min;
    return {
        key,
        min,
        max,
        points: points.map((p) =>
            p == null || !Number.isFinite(p) ? null : range === 0 ? 0.5 : (p - min) / range,
        ),
    };
}

/**
 * Both halves of a pair, normalised and trimmed to the slots where BOTH have a
 * value.
 *
 * Trimming matters: the correlation was computed over paired observations only,
 * so drawing days where one series is absent would show the reader a chart that
 * does not correspond to the number beside it.
 *
 * `null` when fewer than two paired slots survive — one point is not a shape.
 */
export function pairedSeries(
    a: string,
    aPoints: (number | null)[],
    b: string,
    bPoints: (number | null)[],
): { a: NormalizedSeries; b: NormalizedSeries; slots: number } | null {
    const len = Math.min(aPoints.length, bPoints.length);
    const aPaired: (number | null)[] = [];
    const bPaired: (number | null)[] = [];
    for (let i = 0; i < len; i++) {
        const x = aPoints[i];
        const y = bPoints[i];
        const both = x != null && y != null && Number.isFinite(x) && Number.isFinite(y);
        aPaired.push(both ? x : null);
        bPaired.push(both ? y : null);
    }
    const slots = aPaired.filter((p) => p != null).length;
    if (slots < 2) return null;
    const na = normalize(a, aPaired);
    const nb = normalize(b, bPaired);
    return na && nb ? { a: na, b: nb, slots } : null;
}

/** How to describe the relationship's direction in one word. */
export function directionLabel(rho: number): 'together' | 'inversely' {
    return rho >= 0 ? 'together' : 'inversely';
}

/**
 * How much weight the reader should put on a pair, from its sample size.
 *
 * The daemon already gates at n >= 20, so nothing arrives that is outright
 * unreportable — but 26 and 156 are different claims and the UI should say so
 * rather than presenting a flat list of equals.
 */
export function confidence(n: number): 'strong' | 'moderate' | 'thin' {
    if (n >= 100) return 'strong';
    if (n >= 40) return 'moderate';
    return 'thin';
}

/** Sentence for a pair — states the relationship AND its limits. */
export function correlationSummary(c: MetricCorrelation, nameOf: (key: string) => string): string {
    const dir = c.rho >= 0 ? 'rises with' : 'falls as';
    return `${nameOf(c.a)} ${dir} ${nameOf(c.b)} (ρ ${c.rho.toFixed(2)}, ${c.n} paired days). Related, not necessarily causal.`;
}
