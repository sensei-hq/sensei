import { describe, it, expect } from 'vitest';
import {
    formatMetricValue,
    metricTrend,
    metricSub,
    toMetricCard,
    groupByFamily,
    METRIC_NONE,
    toneColor,
    toSignal,
    buildSignals,
    healthSignal,
    heroSignal,
    pickMovers,
    orderSignals,
    deterministicHeadline,
    seriesDistribution,
    densifySeries,
    metricYDomain,
    chartKindForType,
    defaultWindowForGrain,
    movingAverage,
    historyNote,
    linkifyMetrics,
    mostRecentDayWithValue,
    recentDaysWithData,
    explainerForDay,
    sessionOneLiner,
    shortSessionId,
    formatDayLabel,
    type ProjectMetricRow,
    type MetricFamily,
    type MetricSeriesPoint,
    type RegistryMetric,
    type DrilldownSession,
} from './metric-view.js';

function row(over: Partial<ProjectMetricRow>): ProjectMetricRow {
    return {
        metric: 'ftr',
        date: '2026-08-10',
        value: 1,
        props: {},
        name: 'First-turn resolution (FTR)',
        metric_type: 'pct',
        unit: '%',
        direction: 'higher_better',
        purpose: 'north-star',
        how_to_read: 'never read alone',
        prior: null,
        delta: null,
        ...over,
    };
}

describe('formatMetricValue', () => {
    it('renders a percentage from a 0..1 rate', () => {
        expect(formatMetricValue('pct', 0.417)).toBe('42%');
        expect(formatMetricValue('pct', 1)).toBe('100%');
    });
    it('renders a ratio to two decimals', () => {
        expect(formatMetricValue('ratio', 0.166437)).toBe('0.17');
    });
    it('renders a count with thousands separators', () => {
        expect(formatMetricValue('count', 11900)).toBe('11,900');
    });
    it('renders a score as a rounded integer', () => {
        expect(formatMetricValue('score', 83.6)).toBe('84');
    });
    it('renders durations as compact human time', () => {
        expect(formatMetricValue('duration', 20)).toBe('20s');
        expect(formatMetricValue('duration', 80)).toBe('1m 20s');
        expect(formatMetricValue('duration', 3600)).toBe('1h');
        expect(formatMetricValue('duration', 3900)).toBe('1h 5m');
    });
    it('returns the em dash for a missing value, never a fabricated 0', () => {
        expect(formatMetricValue('pct', null)).toBe(METRIC_NONE);
        expect(formatMetricValue('duration', undefined)).toBe(METRIC_NONE);
        expect(formatMetricValue('count', NaN)).toBe(METRIC_NONE);
    });
});

describe('metricTrend', () => {
    it('marks a rise in a higher-better metric as good, up', () => {
        const t = metricTrend('pct', 'higher_better', 0.2857);
        expect(t).toEqual({ label: '+29 pt', dir: 'up', tone: 'good' });
    });
    it('marks a fall in a higher-better metric as bad, down', () => {
        expect(metricTrend('pct', 'higher_better', -0.1)?.tone).toBe('bad');
    });
    it('marks a fall in a lower-better metric as good', () => {
        const t = metricTrend('count', 'lower_better', -11368);
        expect(t).toEqual({ label: '−11,368', dir: 'down', tone: 'good' });
    });
    it('marks a rise in a lower-better metric as bad', () => {
        expect(metricTrend('duration', 'lower_better', 5)).toEqual({
            label: '+5s',
            dir: 'up',
            tone: 'bad',
        });
    });
    it('keeps a neutral-direction change neutral regardless of sign', () => {
        expect(metricTrend('pct', 'neutral', 0.02)?.tone).toBe('neutral');
        expect(metricTrend('pct', 'neutral', -0.02)?.tone).toBe('neutral');
    });
    it('returns null when there is no prior to compare (delta null)', () => {
        expect(metricTrend('pct', 'higher_better', null)).toBeNull();
    });
    it('reads an exact-zero delta as flat/neutral', () => {
        expect(metricTrend('ratio', 'lower_better', 0)).toEqual({
            label: '0',
            dir: 'flat',
            tone: 'neutral',
        });
    });
    it('treats a sub-resolution change as flat, not a confusing "−0.00"', () => {
        // a ratio delta that rounds to 0.00 at 2dp
        expect(metricTrend('ratio', 'lower_better', -0.0005)).toEqual({
            label: '0',
            dir: 'flat',
            tone: 'neutral',
        });
        // a duration delta under half a second
        expect(metricTrend('duration', 'lower_better', 0.3)?.dir).toBe('flat');
    });
});

describe('metricSub', () => {
    it('prefers numerator / denominator when both present', () => {
        expect(metricSub(row({ props: { numerator: 1210, denominator: 7270 } }))).toBe(
            '1,210 / 7,270',
        );
    });
    it('falls back to the contributing session count (n)', () => {
        expect(metricSub(row({ metric_type: 'duration', props: { n: 12 } }))).toBe('12 sessions');
    });
    it('shows the unit for non-pct metrics with no props', () => {
        expect(
            metricSub(row({ metric_type: 'count', unit: 'executions/day', props: {} })),
        ).toBe('executions/day');
    });
    it('is empty for a bare pct metric', () => {
        expect(metricSub(row({ metric_type: 'pct', unit: '%', props: {} }))).toBe('');
    });
});

describe('toMetricCard', () => {
    it('assembles a card from a wire row', () => {
        const card = toMetricCard(
            row({
                metric: 'ftr',
                value: 1,
                props: { numerator: 2, denominator: 2 },
                delta: 0.2857,
            }),
        );
        expect(card.key).toBe('ftr');
        expect(card.value).toBe('100%');
        expect(card.sub).toBe('2 / 2');
        expect(card.trend?.tone).toBe('good');
    });
    it('renders time_to_useful_result as the em dash when the value is absent', () => {
        const card = toMetricCard(
            row({ metric: 'time_to_useful_result', metric_type: 'duration', value: null }),
        );
        expect(card.value).toBe(METRIC_NONE);
        expect(card.trend).toBeNull();
    });
});

describe('groupByFamily', () => {
    const familyOf = (key: string): MetricFamily | undefined =>
        ({
            ftr: 'outcome',
            time_to_useful_result: 'velocity',
            project_health: 'composite',
        })[key] as MetricFamily | undefined;

    it('orders sections composite -> outcome -> velocity and only includes present families', () => {
        const sections = groupByFamily(
            [
                row({ metric: 'ftr' }),
                row({ metric: 'time_to_useful_result', metric_type: 'duration' }),
                row({ metric: 'project_health', metric_type: 'score', value: 84 }),
            ],
            familyOf,
        );
        expect(sections.map((s) => s.family)).toEqual(['composite', 'outcome', 'velocity']);
        expect(sections[0].label).toBe('Health');
    });

    it('files an unknown metric key under tool so it still renders', () => {
        const sections = groupByFamily([row({ metric: 'brand_new_metric' })], () => undefined);
        expect(sections).toHaveLength(1);
        expect(sections[0].family).toBe('tool');
        expect(sections[0].cards[0].key).toBe('brand_new_metric');
    });
});

// ── "The merge" UX signal view-model ────────────────────────────────────────

const famOf = (key: string): MetricFamily | undefined =>
    ({
        project_health: 'composite',
        churn_concentration: 'quality',
        time_to_useful_result: 'velocity',
        unused_tools: 'tool',
    })[key] as MetricFamily | undefined;

describe('toneColor', () => {
    it('maps good→success, bad→accent, neutral/absent→ink-faint (the grid legend)', () => {
        expect(toneColor('good')).toBe('success');
        expect(toneColor('bad')).toBe('accent');
        expect(toneColor('neutral')).toBe('ink-faint');
        expect(toneColor(null)).toBe('ink-faint');
    });
});

describe('toSignal', () => {
    it('reframes unused_tools as "N of M relevant tools used" from props', () => {
        const s = toSignal(
            row({
                metric: 'unused_tools',
                metric_type: 'count',
                direction: 'lower_better',
                value: 3, // unused = M - N
                props: { total_tools: 106, relevant_tools: 12, used_tools: 9 },
                name: 'Unused-tool count',
            }),
            famOf,
        );
        expect(s.name).toBe('Tools used');
        expect(s.value).toBe('9 of 12');
        expect(s.sub).toContain('relevant');
        expect(s.sub).toContain('106 registered');
        expect(s.insight).toContain('9 of 12 relevant tools');
        expect(s.insight).toContain('106 registered');
    });

    it('worsening lower_better metric (value rose) is accent-colored + a mover', () => {
        const s = toSignal(
            row({
                metric: 'churn_concentration',
                metric_type: 'pct',
                direction: 'higher_better',
                value: 0.57,
                prior: 0.36,
                delta: 0.21,
            }),
            famOf,
        );
        // higher_better and rose → improving → success. Flip the direction:
        expect(s.color).toBe('success');
        expect(s.moved).toBe(true);
        const worse = toSignal(
            row({
                metric: 'time_to_useful_result',
                metric_type: 'duration',
                direction: 'lower_better',
                value: 796,
                prior: 273,
                delta: 523,
            }),
            famOf,
        );
        expect(worse.color).toBe('accent'); // duration rose on a lower_better metric
        expect(worse.moved).toBe(true);
    });

    it('deterministic insight states value, num/den, and change — no invented data', () => {
        const s = toSignal(
            row({
                metric: 'churn_concentration',
                name: 'Churn concentration',
                metric_type: 'pct',
                direction: 'higher_better',
                value: 0.57,
                props: { numerator: 190, denominator: 333 },
                prior: 0.36,
                delta: 0.21,
            }),
            famOf,
        );
        expect(s.insight).toBe('Churn concentration is 57% (190 / 333), +21 pt vs the prior period.');
    });

    it('an ollama insight overrides the deterministic sentence when present', () => {
        const s = toSignal(
            row({ metric: 'churn_concentration', metric_type: 'pct', value: 0.57, delta: 0.21 }),
            famOf,
            { insights: { churn_concentration: 'Work is circling the same files.' } },
        );
        expect(s.insight).toBe('Work is circling the same files.');
    });

    it('no prior → no trend chip, first-period insight, not a mover', () => {
        const s = toSignal(row({ metric: 'churn_concentration', value: 0.5, delta: null }), famOf);
        expect(s.trend).toBeNull();
        expect(s.moved).toBe(false);
        expect(s.insight).toContain('first period on record');
    });
});

describe('linkifyMetrics', () => {
    const reg = (key: string, name: string): RegistryMetric => ({
        key,
        name,
        family: 'outcome',
        metric_type: 'ratio',
        unit: null,
        direction: 'higher_better',
    });
    const registry: RegistryMetric[] = [
        reg('rework_ratio', 'Rework ratio'),
        reg('ftr', 'First-turn resolution (FTR)'),
        reg('churn_rate', 'Churn rate'),
    ];

    it('links another metric named in the text (case-insensitive), keeping the prose', () => {
        const segs = linkifyMetrics('Never read alone. Companion: rework ratio.', registry, 'ftr');
        expect(segs.map((s) => s.text).join('')).toBe('Never read alone. Companion: rework ratio.');
        const linked = segs.find((s) => s.key);
        expect(linked?.key).toBe('rework_ratio');
        expect(linked?.text).toBe('rework ratio');
    });

    it('never links the current metric to itself', () => {
        const segs = linkifyMetrics('Rework ratio rises when work is deferred.', registry, 'rework_ratio');
        expect(segs.every((s) => s.key == null)).toBe(true);
    });

    it('returns a single plain segment when nothing matches', () => {
        const segs = linkifyMetrics('no companion mentioned here', registry, 'ftr');
        expect(segs).toEqual([{ text: 'no companion mentioned here' }]);
    });

    it('does not match a name embedded inside a larger word', () => {
        // "Churn rate" must not link inside "churn rated" (word-boundary guard).
        const segs = linkifyMetrics('the churn rated poorly', registry, 'ftr');
        expect(segs.every((s) => s.key == null)).toBe(true);
    });
});

describe('heroSignal', () => {
    it('leads with FTR (the north star) over the composite when both are present', () => {
        const signals = buildSignals(
            [row({ metric: 'ftr', value: 1 }), row({ metric: 'project_health', metric_type: 'score', value: 44 })],
            famOf,
        );
        expect(heroSignal(signals)?.key).toBe('ftr');
    });

    it('is FTR when the composite is retired (no composite row)', () => {
        const signals = buildSignals(
            [row({ metric: 'ftr', value: 1 }), row({ metric: 'churn_concentration', metric_type: 'pct', value: 0.5 })],
            famOf,
        );
        expect(heroSignal(signals)?.key).toBe('ftr');
    });

    it('falls back to the composite only when FTR is absent (legacy data)', () => {
        const signals = buildSignals([row({ metric: 'project_health', metric_type: 'score', value: 44 })], famOf);
        expect(heroSignal(signals)?.key).toBe('project_health');
    });

    it('is null when neither FTR nor a composite exists', () => {
        const signals = buildSignals([row({ metric: 'churn_concentration', metric_type: 'pct', value: 0.5 })], famOf);
        expect(heroSignal(signals)).toBeNull();
    });
});

describe('movers + ordering', () => {
    const rows: ProjectMetricRow[] = [
        row({ metric: 'project_health', metric_type: 'score', value: 44, prior: 46, delta: -2 }),
        row({ metric: 'churn_concentration', metric_type: 'pct', value: 0.57, prior: 0.36, delta: 0.21 }),
        row({ metric: 'time_to_useful_result', metric_type: 'duration', direction: 'lower_better', value: 796, prior: 273, delta: 523 }),
        row({ metric: 'unused_tools', metric_type: 'count', direction: 'lower_better', value: 3, prior: 3, delta: 0, props: { total_tools: 106, relevant_tools: 12, used_tools: 9 } }),
    ];
    const signals = buildSignals(rows, famOf);

    it('health is the hero, never a mover', () => {
        expect(healthSignal(signals)?.key).toBe('project_health');
        expect(pickMovers(signals).some((s) => s.key === 'project_health')).toBe(false);
    });

    it('movers exclude the flat signal and rank by relative magnitude', () => {
        const movers = pickMovers(signals);
        expect(movers.map((s) => s.key)).not.toContain('unused_tools'); // flat (delta 0)
        // time_to_useful (523/273 ≈ 1.92) outranks churn_concentration (0.21/0.36 ≈ 0.58)
        expect(movers[0].key).toBe('time_to_useful_result');
        expect(movers[1].key).toBe('churn_concentration');
    });

    it('grid order is composite first, then movers, then the rest', () => {
        const ordered = orderSignals(signals).map((s) => s.key);
        expect(ordered[0]).toBe('project_health');
        expect(ordered.indexOf('time_to_useful_result')).toBeLessThan(ordered.indexOf('unused_tools'));
    });
});

describe('deterministicHeadline', () => {
    it('counts movers and their direction', () => {
        const signals = buildSignals(
            [
                row({ metric: 'project_health', metric_type: 'score', value: 44, prior: 46, delta: -2 }),
                row({ metric: 'churn_concentration', metric_type: 'pct', direction: 'higher_better', value: 0.57, prior: 0.36, delta: 0.21 }),
                row({ metric: 'time_to_useful_result', metric_type: 'duration', direction: 'lower_better', value: 796, prior: 273, delta: 523 }),
            ],
            famOf,
        );
        const h = deterministicHeadline(signals);
        expect(h).toContain('2 signals moved');
        expect(h).toContain('1 worsening');
        expect(h).toContain('1 improving');
    });

    it('says nothing moved when all are flat', () => {
        const signals = buildSignals([row({ metric: 'churn_concentration', delta: 0 })], famOf);
        expect(deterministicHeadline(signals)).toContain('Nothing moved');
    });
});

describe('seriesDistribution', () => {
    it('returns high/mean/low, or null for an empty series', () => {
        expect(seriesDistribution([])).toBeNull();
        expect(seriesDistribution([0.25, 0.61, 0.43])).toEqual({ high: 0.61, mean: 0.43, low: 0.25 });
    });
});

// ── Datapoint drill-down helpers ─────────────────────────────────────────────

function dp(period: string, value: number | null, explainer?: string | null): MetricSeriesPoint {
    return { period, value, direction: 'higher_better', explainer };
}

describe('mostRecentDayWithValue', () => {
    it('is the latest day carrying a value, ignoring null (gap) points', () => {
        expect(
            mostRecentDayWithValue([dp('2026-08-09', 0.5), dp('2026-08-11', null), dp('2026-08-10', 1)]),
        ).toBe('2026-08-10');
    });
    it('is null when no point has a value (a genuinely empty series)', () => {
        expect(mostRecentDayWithValue([dp('2026-08-10', null)])).toBeNull();
        expect(mostRecentDayWithValue([])).toBeNull();
    });
});

describe('recentDaysWithData', () => {
    it('lists only days with a value, ascending and de-duplicated', () => {
        expect(
            recentDaysWithData([dp('2026-08-11', 1), dp('2026-08-10', null), dp('2026-08-09', 0.5)]),
        ).toEqual(['2026-08-09', '2026-08-11']);
    });
    it('caps to the most-recent `limit` days', () => {
        const series = ['05', '06', '07', '08', '09'].map((d) => dp(`2026-08-${d}`, 1));
        expect(recentDaysWithData(series, 2)).toEqual(['2026-08-08', '2026-08-09']);
    });
    it('is empty for a series with no values (never a fabricated day)', () => {
        expect(recentDaysWithData([dp('2026-08-10', null)])).toEqual([]);
    });
});

describe('explainerForDay', () => {
    const series = [dp('2026-08-11', 1, '  both sessions landed first-try  '), dp('2026-08-10', 0.5, '')];
    it('returns the point explainer for the day, trimmed', () => {
        expect(explainerForDay(series, '2026-08-11')).toBe('both sessions landed first-try');
    });
    it('is null for a day with no point, no explainer, or no day', () => {
        expect(explainerForDay(series, '2026-08-10')).toBeNull(); // empty explainer
        expect(explainerForDay(series, '2026-08-01')).toBeNull(); // no point
        expect(explainerForDay(series, null)).toBeNull();
        expect(explainerForDay([dp('2026-08-11', 1)], '2026-08-11')).toBeNull(); // no explainer field
    });
});

describe('sessionOneLiner', () => {
    function s(over: Partial<DrilldownSession> = {}): DrilldownSession {
        return {
            client_session_id: 'abc',
            started_at: '2026-08-11T09:00:00Z',
            outcome: 'completed',
            ftr: true,
            turns: 3,
            corrections: 0,
            task: 't',
            summary: null,
            observation: { title: 'x', detail: 'y' },
            evidence: null,
            resumed: false,
            trouble: null,
            ...over,
        };
    }
    it('reads a first-try session as "outcome · first-try · N turns"', () => {
        expect(sessionOneLiner(s())).toBe('completed · first-try · 3 turns');
    });
    it('reads a corrected session as its correction count (pluralized)', () => {
        expect(sessionOneLiner(s({ ftr: false, corrections: 2, turns: 4, outcome: 'corrected' }))).toBe(
            'corrected · 2 corrections · 4 turns',
        );
        expect(sessionOneLiner(s({ ftr: false, corrections: 1, turns: 1 }))).toBe(
            'completed · 1 correction · 1 turn',
        );
        expect(sessionOneLiner(s({ ftr: false, corrections: 0 }))).toContain('no corrections');
    });
    it('omits the outcome segment when the outcome is absent', () => {
        expect(sessionOneLiner(s({ outcome: null }))).toBe('first-try · 3 turns');
    });
});

describe('shortSessionId', () => {
    it('is the first 8 chars of the id', () => {
        expect(shortSessionId('abcdef1234567890')).toBe('abcdef12');
    });
    it('is the em dash for an absent id (never a fabricated id)', () => {
        expect(shortSessionId(null)).toBe(METRIC_NONE);
        expect(shortSessionId('')).toBe(METRIC_NONE);
    });
});

describe('formatDayLabel', () => {
    it('renders a compact, locale-free label', () => {
        expect(formatDayLabel('2026-08-11')).toBe('Aug 11');
        expect(formatDayLabel('2026-01-01T00:00:00Z')).toBe('Jan 1');
    });
    it('falls back to the raw ISO when unparseable', () => {
        expect(formatDayLabel('not-a-date')).toBe('not-a-date');
    });
});

// ── Chart series shape ───────────────────────────────────────────────────────

function pt(period: string, value: number | null): MetricSeriesPoint {
    return { period, value, direction: 'higher_better' };
}

describe('densifySeries', () => {
    it('is empty for no points', () => {
        expect(densifySeries([], 'daily')).toEqual([]);
    });

    it('keeps consecutive daily points as-is, preserving a genuine 0', () => {
        expect(densifySeries([pt('2025-01-01', 0), pt('2025-01-02', 3)], 'daily')).toEqual([
            { date: '2025-01-01', value: 0 },
            { date: '2025-01-02', value: 3 },
        ]);
    });

    it('inserts a null for every absent day so a gap is not connected or zero-filled', () => {
        expect(densifySeries([pt('2025-01-01', 5), pt('2025-01-04', 7)], 'daily')).toEqual([
            { date: '2025-01-01', value: 5 },
            { date: '2025-01-02', value: null },
            { date: '2025-01-03', value: null },
            { date: '2025-01-04', value: 7 },
        ]);
    });

    it('renders a multi-month lull as a run of gaps, not a line to 0', () => {
        const out = densifySeries([pt('2025-06-05', 1), pt('2025-09-04', 1)], 'daily');
        expect(out.at(0)).toEqual({ date: '2025-06-05', value: 1 });
        expect(out.at(-1)).toEqual({ date: '2025-09-04', value: 1 });
        // Every interior slot is an absent day (a gap), never a fabricated 0.
        expect(out.slice(1, -1).every((p) => p.value === null)).toBe(true);
        expect(out.length).toBe(92); // Jun 5 → Sep 4 inclusive
    });

    it('steps by week / month for coarser grains', () => {
        expect(densifySeries([pt('2025-01-06', 1), pt('2025-01-20', 2)], 'weekly')).toEqual([
            { date: '2025-01-06', value: 1 },
            { date: '2025-01-13', value: null },
            { date: '2025-01-20', value: 2 },
        ]);
        expect(densifySeries([pt('2025-01-01', 1), pt('2025-04-01', 2)], 'monthly')).toEqual([
            { date: '2025-01-01', value: 1 },
            { date: '2025-02-01', value: null },
            { date: '2025-03-01', value: null },
            { date: '2025-04-01', value: 2 },
        ]);
    });
});

describe('metricYDomain', () => {
    it('pins a rate to a full 0–1 axis so a flat pct looks flat', () => {
        expect(metricYDomain('pct', [0.417, 0.42, 0.41])).toEqual([0, 1]);
    });
    it('pins a near-flat ratio to 0-based [0,1] (no 0.128–0.130 mountain)', () => {
        expect(metricYDomain('ratio', [0.128, 0.13, 0.129])).toEqual([0, 1]);
    });
    it('extends a ratio ceiling only when the data exceeds 1 (never clips)', () => {
        expect(metricYDomain('ratio', [0.5, 2.5])).toEqual([0, 2.5]);
    });
    it('uses a 0–100 scale for a score', () => {
        expect(metricYDomain('score', [44, 46])).toEqual([0, 100]);
    });
    it('is 0-based to the data max for counts and durations', () => {
        expect(metricYDomain('count', [3, 7, 4])).toEqual([0, 7]);
        expect(metricYDomain('duration', [796, 273])).toEqual([0, 796]);
    });
    it('falls back to a unit ceiling for an all-zero / empty series', () => {
        expect(metricYDomain('count', [0, 0])).toEqual([0, 1]);
        expect(metricYDomain('count', [])).toEqual([0, 1]);
    });
});

describe('historyNote', () => {
    it('captions the first reading as the metric horizon', () => {
        expect(historyNote([{ date: '2025-06-05', value: 1 }, { date: '2025-06-06', value: 2 }])).toBe(
            'history from 2025-06-05 — no earlier data for this signal',
        );
    });
    it('skips leading gaps to the first real reading', () => {
        expect(
            historyNote([
                { date: '2025-06-05', value: null },
                { date: '2025-06-06', value: 4 },
            ]),
        ).toBe('history from 2025-06-06 — no earlier data for this signal');
    });
    it('is empty when there is no reading yet', () => {
        expect(historyNote([])).toBe('');
    });

    describe('chart geom + window helpers', () => {
        it('picks bars for counts and a line for everything else', () => {
            expect(chartKindForType('count')).toBe('bar');
            expect(chartKindForType('pct')).toBe('line');
            expect(chartKindForType('ratio')).toBe('line');
            expect(chartKindForType('duration')).toBe('line');
            expect(chartKindForType('score')).toBe('line');
        });

        it('defaults the window per grain (7d / 4w / 3mo, else all)', () => {
            expect(defaultWindowForGrain('daily')).toBe(7);
            expect(defaultWindowForGrain('weekly')).toBe(4);
            expect(defaultWindowForGrain('monthly')).toBe(3);
            expect(defaultWindowForGrain('quarterly')).toBeNull();
        });

        it('moving-averages the trend, keeping nulls as gaps', () => {
            const s = [
                { date: 'a', value: 2 },
                { date: 'b', value: 4 },
                { date: 'c', value: null },
                { date: 'd', value: 6 },
            ];
            const ma = movingAverage(s, 3);
            expect(ma[0]).toBe(3); // (2+4)/2
            expect(ma[1]).toBe(3); // (2+4)/2 (c is null, skipped)
            expect(ma[2]).toBeNull(); // the gap stays a gap
            expect(ma[3]).toBe(6); // (6)/1
        });
    });
});
