// Pure view logic for project metrics: wire rows -> presentational card models.
//
// The daemon serves two shapes (see crates/senseid): the per-project *values*
// (`GET /api/projects/{id}/metrics` -> ProjectMetricRow[]) and the *catalog*
// (`GET /api/metrics/registry`) which alone carries each metric's `family`. This
// module keeps all formatting/trend/grouping decisions in one testable place so
// the components stay dumb templates and the loader stays a thin fetch+join.

/** Wire enum: how a metric's `value` should be read. */
export type MetricType = 'pct' | 'ratio' | 'count' | 'duration' | 'score';
/** Wire enum: which direction of change is an improvement. */
export type MetricDirection = 'higher_better' | 'lower_better' | 'neutral';
/** Registry facet used to cluster the metrics page into sections. */
export type MetricFamily =
    | 'composite'
    | 'outcome'
    | 'velocity'
    | 'quality'
    | 'knowledge'
    | 'autonomy'
    | 'tool';

/** One row of `GET /api/projects/{id}/metrics` (ProjectMetricRow + injected trend). */
export interface ProjectMetricRow {
    metric: string;
    date: string;
    value: number | null;
    props: Record<string, unknown> | null;
    name: string;
    metric_type: MetricType;
    unit: string | null;
    direction: MetricDirection;
    purpose: string;
    how_to_read: string;
    prior: number | null;
    delta: number | null;
}

export const FAMILY_ORDER: MetricFamily[] = [
    'composite',
    'outcome',
    'velocity',
    'quality',
    'knowledge',
    'autonomy',
    'tool',
];

export const FAMILY_LABEL: Record<MetricFamily, string> = {
    composite: 'Health',
    outcome: 'Outcome',
    velocity: 'Velocity',
    quality: 'Quality',
    knowledge: 'Knowledge',
    autonomy: 'Autonomy',
    tool: 'Tooling',
};

/** The no-data marker — shown when a metric has no value yet (never a fake 0). */
export const METRIC_NONE = '—';

export type TrendTone = 'good' | 'bad' | 'neutral';

export interface MetricTrend {
    label: string; // e.g. "+29 pt", "−5s"
    dir: 'up' | 'down' | 'flat';
    tone: TrendTone;
}

export interface MetricCardVM {
    key: string;
    name: string;
    value: string; // formatted, or METRIC_NONE
    sub: string; // short context line (unit / n / numerator·denominator)
    trend: MetricTrend | null;
    howToRead: string;
    purpose: string;
}

export interface MetricSection {
    family: MetricFamily;
    label: string;
    cards: MetricCardVM[];
}

const MINUS = '−'; // proper minus sign, not a hyphen

function formatDuration(seconds: number): string {
    const s = Math.round(seconds);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) {
        const rem = s % 60;
        return rem ? `${m}m ${rem}s` : `${m}m`;
    }
    const h = Math.floor(m / 60);
    const remM = m % 60;
    return remM ? `${h}h ${remM}m` : `${h}h`;
}

function formatCount(value: number): string {
    return Math.round(value).toLocaleString('en-US');
}

function formatRatio(value: number): string {
    return value.toFixed(2);
}

/** Format a metric's headline value by its wire type. Absent -> METRIC_NONE. */
export function formatMetricValue(
    type: MetricType,
    value: number | null | undefined,
): string {
    if (value == null || Number.isNaN(value)) return METRIC_NONE;
    switch (type) {
        case 'pct':
            return `${Math.round(value * 100)}%`;
        case 'score':
            return `${Math.round(value)}`;
        case 'count':
            return formatCount(value);
        case 'duration':
            return formatDuration(value);
        case 'ratio':
            return formatRatio(value);
        default:
            return METRIC_NONE;
    }
}

/** Format the magnitude of a change, in the metric's own units. */
function formatDeltaMagnitude(type: MetricType, magnitude: number): string {
    switch (type) {
        case 'pct':
            return `${Math.round(magnitude * 100)} pt`;
        case 'score':
            return `${Math.round(magnitude)}`;
        case 'count':
            return formatCount(magnitude);
        case 'duration':
            return formatDuration(magnitude);
        case 'ratio':
            return formatRatio(magnitude);
        default:
            return `${magnitude}`;
    }
}

/**
 * Derive the trend chip from `delta` + `direction`. Returns null when there is
 * no prior to compare against (delta null) — the card then shows no chip rather
 * than implying a flat line. A genuine zero delta reads as "flat / neutral".
 */
export function metricTrend(
    type: MetricType,
    direction: MetricDirection,
    delta: number | null | undefined,
): MetricTrend | null {
    if (delta == null || Number.isNaN(delta)) return null;
    if (delta === 0) return { label: '0', dir: 'flat', tone: 'neutral' };

    // A change that rounds to nothing at the metric's display resolution reads
    // as flat — never a confusing "−0.00" / "+0s".
    const magnitude = formatDeltaMagnitude(type, Math.abs(delta));
    if (magnitude === formatDeltaMagnitude(type, 0)) {
        return { label: '0', dir: 'flat', tone: 'neutral' };
    }

    const up = delta > 0;
    const sign = up ? '+' : MINUS;
    const label = `${sign}${magnitude}`;

    let tone: TrendTone = 'neutral';
    if (direction === 'higher_better') tone = up ? 'good' : 'bad';
    else if (direction === 'lower_better') tone = up ? 'bad' : 'good';

    return { label, dir: up ? 'up' : 'down', tone };
}

function numProp(props: Record<string, unknown> | null, key: string): number | null {
    const v = props?.[key];
    return typeof v === 'number' ? v : null;
}

/** A short context line under the value — surfaced from `props`/`unit`. */
export function metricSub(row: ProjectMetricRow): string {
    const numerator = numProp(row.props, 'numerator');
    const denominator = numProp(row.props, 'denominator');
    if (numerator != null && denominator != null) {
        return `${formatCount(numerator)} / ${formatCount(denominator)}`;
    }
    const n = numProp(row.props, 'n');
    if (n != null) return `${formatCount(n)} sessions`;
    if (row.unit && row.metric_type !== 'pct') return row.unit;
    return '';
}

/** Map one wire row to a presentational card model. */
export function toMetricCard(row: ProjectMetricRow): MetricCardVM {
    return {
        key: row.metric,
        name: row.name,
        value: formatMetricValue(row.metric_type, row.value),
        sub: metricSub(row),
        trend: metricTrend(row.metric_type, row.direction, row.delta),
        howToRead: row.how_to_read,
        purpose: row.purpose,
    };
}

/**
 * Group rows into ordered family sections. `familyOf` resolves a metric key to
 * its family (from the registry join); unknown keys fall into 'tool' so a new
 * daemon metric still renders rather than vanishing.
 */
export function groupByFamily(
    rows: ProjectMetricRow[],
    familyOf: (key: string) => MetricFamily | undefined,
): MetricSection[] {
    const byFamily = new Map<MetricFamily, MetricCardVM[]>();
    for (const row of rows) {
        const family = familyOf(row.metric) ?? 'tool';
        const cards = byFamily.get(family) ?? [];
        cards.push(toMetricCard(row));
        byFamily.set(family, cards);
    }
    return FAMILY_ORDER.filter((family) => byFamily.has(family)).map((family) => ({
        family,
        label: FAMILY_LABEL[family],
        cards: byFamily.get(family) ?? [],
    }));
}

/** One row of `GET /api/metrics/registry` — the catalog that carries `family`. */
export interface RegistryMetric {
    key: string;
    family: MetricFamily;
    name: string;
    metric_type: MetricType;
    unit: string | null;
    direction: MetricDirection;
}

/** One point of `GET /api/projects/{id}/metrics/{key}?grain=…`. */
export interface MetricSeriesPoint {
    period: string;
    value: number | null;
    direction: MetricDirection;
}

/** Build a key→family resolver from the registry catalog (the values endpoint
 *  omits `family`, so grouping needs this join). */
export function familyLookup(
    registry: RegistryMetric[],
): (key: string) => MetricFamily | undefined {
    const map = new Map(registry.map((r) => [r.key, r.family]));
    return (key) => map.get(key);
}

/** The plottable numeric values of a series, dropping any null points. */
export function seriesValues(points: MetricSeriesPoint[]): number[] {
    return points.map((p) => p.value).filter((v): v is number => v != null);
}
