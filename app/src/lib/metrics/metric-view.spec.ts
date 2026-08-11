import { describe, it, expect } from 'vitest';
import {
    formatMetricValue,
    metricTrend,
    metricSub,
    toMetricCard,
    groupByFamily,
    METRIC_NONE,
    type ProjectMetricRow,
    type MetricFamily,
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
