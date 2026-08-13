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
    if (h < 24) {
        const remM = m % 60;
        return remM ? `${h}h ${remM}m` : `${h}h`;
    }
    // Large durations read as days/hours (e.g. "2025m" → "1d 9h"), not raw hours.
    const d = Math.floor(h / 24);
    const remH = h % 24;
    return remH ? `${d}d ${remH}h` : `${d}d`;
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

/** One row of `GET /api/metrics/registry` — the catalog that carries `family`
 *  and the metric-definition reference facets (`purpose`/`how_to_read`/`formula`)
 *  the "about this metric" block reads. The facets are optional here only so the
 *  type tolerates a pre-facets daemon; the current daemon always sends them. */
export interface RegistryMetric {
    key: string;
    family: MetricFamily;
    name: string;
    metric_type: MetricType;
    unit: string | null;
    direction: MetricDirection;
    purpose?: string;
    how_to_read?: string;
    formula?: string | null;
}

/** One point of `GET /api/projects/{id}/metrics/{key}?grain=…`. */
export interface MetricSeriesPoint {
    period: string;
    value: number | null;
    direction: MetricDirection;
}

/**
 * The static "about this metric" reference block for the detail screen: what it
 * tells you (`purpose`), how to read it (`how_to_read` — interpretation + the
 * companion metric + the read-it-alone gotcha), and how it's computed
 * (`formula`). `purpose`/`how_to_read` ride the per-project row; `formula` rides
 * the series response. Returns `null` when the metric carries no reference text
 * at all — the section then renders nothing rather than an empty shell. Never
 * fabricated: every field is the registry's own copy or absent.
 */
export interface MetricAbout {
    purpose: string;
    howToRead: string;
    formula: string | null;
}

export function metricAbout(
    row: ProjectMetricRow | undefined,
    formula: string | null | undefined,
): MetricAbout | null {
    const purpose = row?.purpose?.trim() ?? '';
    const howToRead = row?.how_to_read?.trim() ?? '';
    const f = formula?.trim() ? formula.trim() : null;
    if (!purpose && !howToRead && !f) return null;
    return { purpose, howToRead, formula: f };
}

/** One run of `how_to_read` text: plain, or (when `key` is set) a mention of
 *  another metric that should render as a link to that metric's detail. */
export interface TextSegment {
    text: string;
    key?: string;
}

/**
 * Split `text` into segments, tagging any run that matches ANOTHER metric's
 * display name (case-insensitive, whole-word) with that metric's key — so the
 * "how to read it" copy can link its companion metric (e.g. FTR's "Companion:
 * rework ratio" links "rework ratio" to the rework_ratio detail). Never links
 * the current metric to itself. Longest names are matched first so a longer name
 * isn't shadowed by a shorter substring. Returns a single plain segment when
 * nothing matches — never fabricates a link.
 */
export function linkifyMetrics(
    text: string,
    registry: RegistryMetric[],
    selfKey: string,
): TextSegment[] {
    const targets = registry
        .filter((m) => m.key !== selfKey && (m.name?.trim().length ?? 0) >= 3)
        .map((m) => {
            const name = m.name.trim();
            return { key: m.key, name, lower: name.toLowerCase() };
        })
        .sort((a, b) => b.name.length - a.name.length); // longest first
    if (!text || targets.length === 0) return [{ text }];

    // Scan by hand (no dynamic RegExp — that's a ReDoS-class pattern, and these
    // names come from the catalog anyway). At each index, try to match the
    // longest target name whose word-ish ends don't sit inside a larger word.
    const lower = text.toLowerCase();
    const isWordChar = (c: string | undefined) => c != null && /[a-z0-9]/i.test(c);
    const segments: TextSegment[] = [];
    let i = 0;
    let plainStart = 0;
    while (i < text.length) {
        let hitKey: string | null = null;
        let hitLen = 0;
        for (const t of targets) {
            if (!lower.startsWith(t.lower, i)) continue;
            const before = i > 0 ? text[i - 1] : undefined;
            const after = text[i + t.name.length];
            const leftOk = !isWordChar(t.name[0]) || !isWordChar(before);
            const rightOk = !isWordChar(t.name[t.name.length - 1]) || !isWordChar(after);
            if (leftOk && rightOk) {
                hitKey = t.key;
                hitLen = t.name.length;
                break; // targets are longest-first, so the first hit is the longest
            }
        }
        if (hitKey) {
            if (i > plainStart) segments.push({ text: text.slice(plainStart, i) });
            segments.push({ text: text.slice(i, i + hitLen), key: hitKey });
            i += hitLen;
            plainStart = i;
        } else {
            i++;
        }
    }
    if (plainStart < text.length) segments.push({ text: text.slice(plainStart) });
    return segments.length ? segments : [{ text }];
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

// ── Chart series shape (consumed by @rokkit/chart) ──────────────────────────
// The daemon returns a *sparse* series — one row per period that has data (see
// PgStore::get_project_metric_series). Absent periods are simply missing, so a
// naïve line connects across a multi-month lull as if it were one segment (or a
// dive to 0). To render that lull as a real GAP, we densify the calendar at the
// series' grain and mark every absent period `value: null`. @rokkit/chart's
// Plot.Line / Plot.Area break the path on a null datum (d3 `.defined`), so a
// null reads as a broken line — while a genuine `0` (a period that had data,
// e.g. ftr 0/1) stays a plotted point. Never zero-fill an absent period.

/** The grains a series can be read at (matches the daemon allowlist). */
export type SeriesGrain = 'daily' | 'weekly' | 'monthly' | 'quarterly';

/** One densified point: an ISO date and its value, or `null` for an absent period. */
export interface ChartPoint {
    date: string;
    value: number | null;
}

/** The next period boundary after `iso` at the given grain (UTC, no locale). */
function nextPeriod(iso: string, grain: SeriesGrain): string {
    const [y, m, d] = iso.slice(0, 10).split('-').map(Number);
    const dt = new Date(Date.UTC(y, (m ?? 1) - 1, d ?? 1));
    switch (grain) {
        case 'weekly':
            dt.setUTCDate(dt.getUTCDate() + 7);
            break;
        case 'monthly':
            dt.setUTCMonth(dt.getUTCMonth() + 1);
            break;
        case 'quarterly':
            dt.setUTCMonth(dt.getUTCMonth() + 3);
            break;
        default:
            dt.setUTCDate(dt.getUTCDate() + 1);
    }
    return dt.toISOString().slice(0, 10);
}

/**
 * Densify a sparse series into one point per period, inserting `value: null`
 * for every absent period between two present ones — so a lull renders as a gap
 * (proportional to its length) rather than a connected segment. Present points
 * are preserved exactly, including genuine `0` values.
 */
export function densifySeries(points: MetricSeriesPoint[], grain: SeriesGrain): ChartPoint[] {
    const clean = points.filter((p) => typeof p.period === 'string' && p.period.length >= 10);
    if (clean.length === 0) return [];
    const CAP = 1000; // never expand a malformed range unboundedly
    const out: ChartPoint[] = [];
    for (let i = 0; i < clean.length; i++) {
        const date = clean[i].period.slice(0, 10);
        out.push({ date, value: clean[i].value });
        const next = i + 1 < clean.length ? clean[i + 1].period.slice(0, 10) : null;
        if (!next) continue;
        let cursor = nextPeriod(date, grain);
        let guard = 0;
        while (cursor < next && guard < CAP) {
            out.push({ date: cursor, value: null });
            cursor = nextPeriod(cursor, grain);
            guard++;
        }
    }
    return out;
}

/**
 * A sensible y-domain per metric type so a flat series looks flat (never a
 * mountain from auto-scaling to a 0.002 spread). Always 0-based (a floor of 0,
 * or below only when the data itself goes negative so a real value is never
 * clipped); bounded metrics read against their natural full scale.
 *   pct   → [0, 1]        (a rate on a full 0–100% axis)
 *   score → [0, 100]      (a 0–100 index)
 *   ratio → [0, 1]        (extended past 1 only when the data exceeds it)
 *   count / duration → [0, data max]
 */
export function metricYDomain(type: MetricType, values: number[]): [number, number] {
    const nums = values.filter((v) => Number.isFinite(v));
    const max = nums.length ? Math.max(...nums) : 0;
    const min = nums.length ? Math.min(...nums) : 0;
    const lo = Math.min(0, min);
    switch (type) {
        case 'pct':
            return [lo, Math.max(1, max)];
        case 'score':
            return [lo, Math.max(100, max)];
        case 'ratio':
            return [lo, Math.max(1, max)];
        case 'count':
        case 'duration':
        default:
            return [lo, max > lo ? max : lo + 1];
    }
}

/**
 * A small horizon annotation for the detail chart: a metric whose source has no
 * earlier data starts its series late, so we caption the first reading rather
 * than leaving a bare late start. Empty string when there is no reading yet.
 */
export function historyNote(series: ChartPoint[]): string {
    const first = series.find((p) => p.value != null)?.date;
    return first ? `history from ${first} — no earlier data for this signal` : '';
}

// ── "The merge" UX: interpreted landing + master-detail drill-down ──────────
// A single richer view-model per metric (a "signal") carries everything the
// landing (health hero + movers + uniform grid) and the detail view need, so
// the components stay dumb. Trend → color follows the mockup legend:
//   improving → success · worsening → accent · flat → ink-faint
// honoring each metric's own `direction` (a rising `lower_better` metric is
// worsening). Nothing here fabricates: the deterministic insight is built
// strictly from the row's real value/props; an ollama-generated sentence, when
// present, overrides it, and its absence falls back — never a made-up number.

/** Named-token stem for a trend rule / sparkline stroke (see legend above). */
export type TrendColor = 'success' | 'accent' | 'ink-faint';

/** Optional daemon-generated prose (headline + per-signal insight). Absent when
 *  the model was unavailable — the UI then shows the deterministic sentence. */
export interface MetricsNarrative {
    headline?: string | null;
    /** A secondary sentence under the headline (generated only; no fabricated
     *  fallback — the headline stands alone when this is absent). */
    subhead?: string | null;
    /** metric key → one "what sensei noticed" sentence. */
    insights?: Record<string, string> | null;
}

export interface SignalVM {
    key: string;
    name: string; // display name (the tools signal is relabelled "Tools used")
    family: MetricFamily;
    familyLabel: string;
    type: MetricType;
    direction: MetricDirection;
    value: string; // formatted headline value, or METRIC_NONE
    sub: string; // numerator/denominator or unit context under the value
    trend: MetricTrend | null;
    color: TrendColor; // rule + sparkline tone
    moved: boolean; // a non-flat trend → a "mover"
    magnitude: number; // unit-free rank key for ordering movers
    insight: string; // one data-grounded sentence (ollama override when present)
    rawValue: number | null;
    rawDelta: number | null;
}

/** The registry key of the tool-relevance metric — relabelled to a "used" framing. */
const KEY_UNUSED_TOOLS = 'unused_tools';

/** Map a trend tone to its rule/sparkline color token. */
export function toneColor(tone: TrendTone | null | undefined): TrendColor {
    if (tone === 'good') return 'success';
    if (tone === 'bad') return 'accent';
    return 'ink-faint';
}

// Literal class maps — UnoCSS extracts utilities by scanning source statically,
// so a dynamic `text-{color}` would never be generated. Keeping the full class
// strings here (one place, scanned) lets components index by TrendColor.
export const TREND_TEXT: Record<TrendColor, string> = {
    success: 'text-success',
    accent: 'text-accent',
    'ink-faint': 'text-ink-faint',
};
/** Top-rule per signal: movers carry their colour, flat signals a hairline. */
export const TREND_RULE: Record<TrendColor, string> = {
    success: 'border-t-success',
    accent: 'border-t-accent',
    'ink-faint': 'border-t-paper-edge',
};

/** Unit-free magnitude for ranking movers: relative change vs the prior, so
 *  metrics in different units are comparable. Falls back to |delta| when there's
 *  no usable prior, and 0 when there's no delta at all (never a mover). */
function moverMagnitude(delta: number | null, prior: number | null): number {
    if (delta == null || Number.isNaN(delta)) return 0;
    if (prior != null && Math.abs(prior) > 1e-9) return Math.abs(delta) / Math.abs(prior);
    return Math.abs(delta);
}

/** The tools signal is shown as "N of M relevant tools used" — flipping the
 *  stored `unused` value (M − N) into the adoption framing the user asked for.
 *  Returns null for every other metric (they use the generic formatting). */
function toolsDisplay(
    props: Record<string, unknown> | null,
): { name: string; value: string; sub: string; insight: string } | null {
    const relevant = numProp(props, 'relevant_tools');
    const used = numProp(props, 'used_tools');
    if (relevant == null || used == null) return null;
    const total = numProp(props, 'total_tools');
    const totalNote = total != null ? ` · ${formatCount(total)} registered` : '';
    return {
        name: 'Tools used',
        value: `${formatCount(used)} of ${formatCount(relevant)}`,
        sub: `relevant${totalNote}`,
        insight:
            `${formatCount(used)} of ${formatCount(relevant)} relevant tools were used` +
            (total != null ? `, of ${formatCount(total)} registered in scope.` : '.'),
    };
}

/** A deterministic, data-grounded sentence for a signal — the honest fallback
 *  when no ollama narrative is available. Never invents a noun or a number: it
 *  states the metric's own value, its numerator/denominator when present, and
 *  its change vs the prior period. */
function deterministicInsight(row: ProjectMetricRow, value: string, sub: string): string {
    if (value === METRIC_NONE) return `${row.name} has no reading for this period yet.`;
    const base = sub ? `${row.name} is ${value} (${sub})` : `${row.name} is ${value}`;
    const t = metricTrend(row.metric_type, row.direction, row.delta);
    if (t == null) return `${base} — the first period on record.`;
    if (t.dir === 'flat') return `${base}, unchanged from the prior period.`;
    return `${base}, ${t.label} vs the prior period.`;
}

/** Map one wire row (+ registry family + optional narrative) to a signal VM. */
export function toSignal(
    row: ProjectMetricRow,
    familyOf: (key: string) => MetricFamily | undefined,
    narrative?: MetricsNarrative | null,
): SignalVM {
    const family = familyOf(row.metric) ?? 'tool';
    const trend = metricTrend(row.metric_type, row.direction, row.delta);
    const tools = row.metric === KEY_UNUSED_TOOLS ? toolsDisplay(row.props) : null;

    const value = tools ? tools.value : formatMetricValue(row.metric_type, row.value);
    const sub = tools ? tools.sub : metricSub(row);
    const name = tools ? tools.name : row.name;

    const generated = narrative?.insights?.[row.metric];
    const insight = generated ?? tools?.insight ?? deterministicInsight(row, value, sub);

    return {
        key: row.metric,
        name,
        family,
        familyLabel: FAMILY_LABEL[family],
        type: row.metric_type,
        direction: row.direction,
        value,
        sub,
        trend,
        color: toneColor(trend?.tone),
        moved: trend != null && trend.dir !== 'flat',
        magnitude: moverMagnitude(row.delta, row.prior),
        insight,
        rawValue: row.value,
        rawDelta: row.delta,
    };
}

/** Build every signal VM. The composite/health signal is the hero, so it is not
 *  a "mover"; everything else is ordered movers-first for the uniform grid. */
export function buildSignals(
    rows: ProjectMetricRow[],
    familyOf: (key: string) => MetricFamily | undefined,
    narrative?: MetricsNarrative | null,
): SignalVM[] {
    return rows.map((r) => toSignal(r, familyOf, narrative));
}

/** The health (composite) signal, promoted to the hero. Null when absent. */
export function healthSignal(signals: SignalVM[]): SignalVM | null {
    return signals.find((s) => s.family === 'composite') ?? null;
}

/** The registry key of the north-star outcome metric. */
const KEY_FTR = 'ftr';

/** The hero signal for the metrics landing. FTR is the north star, so it leads;
 *  the retired `project_health` composite is a fallback only for legacy data that
 *  still carries it. Null when neither is present. (The composite was retired —
 *  a rising-then-falling amalgam that obscured FTR — so the hero leads on the
 *  metric everything is judged by.) */
export function heroSignal(signals: SignalVM[]): SignalVM | null {
    return (
        signals.find((s) => s.key === KEY_FTR) ??
        signals.find((s) => s.family === 'composite') ??
        null
    );
}

/** The top-N movers (non-flat), most-moved first — excludes the composite/health
 *  signal (it is the hero, not a mover). */
export function pickMovers(signals: SignalVM[], n = 4): SignalVM[] {
    return signals
        .filter((s) => s.moved && s.family !== 'composite')
        .sort((a, b) => b.magnitude - a.magnitude)
        .slice(0, n);
}

/** Grid order for "all signals": composite/health first, then movers (by
 *  magnitude), then the rest by family order — so what moved reads first. */
export function orderSignals(signals: SignalVM[]): SignalVM[] {
    const rank = (s: SignalVM): number => {
        if (s.family === 'composite') return 0;
        if (s.moved) return 1;
        return 2;
    };
    return [...signals].sort((a, b) => {
        const ra = rank(a);
        const rb = rank(b);
        if (ra !== rb) return ra - rb;
        if (ra === 1) return b.magnitude - a.magnitude; // movers: biggest first
        return FAMILY_ORDER.indexOf(a.family) - FAMILY_ORDER.indexOf(b.family);
    });
}

/** A one-line deterministic summary headline — the honest fallback when no
 *  ollama headline is present. States how many signals moved and which way. */
export function deterministicHeadline(signals: SignalVM[]): string {
    const movers = signals.filter((s) => s.moved && s.family !== 'composite');
    if (movers.length === 0) return 'Nothing moved this period — the signals are holding steady.';
    const worse = movers.filter((s) => s.color === 'accent').length;
    const better = movers.filter((s) => s.color === 'success').length;
    // A neutral-direction metric can move without being better or worse — count it
    // as "mixed" so the breakdown sums to the mover total (no 6-≠-3+2 mismatch).
    const mixed = movers.length - worse - better;
    const parts: string[] = [];
    if (worse) parts.push(`${worse} worsening`);
    if (better) parts.push(`${better} improving`);
    if (mixed) parts.push(`${mixed} mixed`);
    const n = movers.length;
    const noun = n === 1 ? 'signal' : 'signals';
    return `${n} ${noun} moved this period — ${parts.join(', ')}.`;
}

/** Distribution of a series for the detail "In this period" readout + y-axis. */
export interface SeriesDistribution {
    high: number;
    mean: number;
    low: number;
}

export function seriesDistribution(values: number[]): SeriesDistribution | null {
    if (values.length === 0) return null;
    const high = Math.max(...values);
    const low = Math.min(...values);
    const mean = values.reduce((a, b) => a + b, 0) / values.length;
    return { high, mean, low };
}
