// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import AboutMetricHarness from './AboutMetric.harness.svelte';
import type { MetricAbout } from '$lib/metrics/metric-view.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});

function about(over: Partial<MetricAbout> = {}): MetricAbout {
    return {
        purpose: 'the share of sessions resolved on the first try',
        howToRead: 'read with rework ratio — high ftr with high rework hides churn',
        formula: 'first-try sessions ÷ measurable sessions',
        ...over,
    };
}

function mount(a: MetricAbout) {
    const m = mountComponent(AboutMetricHarness, { about: a });
    cleanup.push(m.destroy);
    return m.container;
}

const q = (root: HTMLElement, sel: string) => root.querySelector(sel) as HTMLElement | null;

describe('AboutMetric', () => {
    it('renders the purpose, how-to-read and formula rows', () => {
        const root = mount(about());
        expect(q(root, '[data-row="purpose"]')?.textContent).toContain('first try');
        expect(q(root, '[data-row="how"]')?.textContent).toContain('rework ratio');
        expect(q(root, '[data-row="formula"]')?.textContent).toContain('÷');
    });

    it('renders the formula in a mono face (it is a computation, not prose)', () => {
        expect(q(mount(about()), '[data-row="formula"]')?.className).toContain('mono');
    });

    it('omits the formula row when the metric has no formula (never an empty shell)', () => {
        expect(q(mount(about({ formula: null })), '[data-row="formula"]')).toBeNull();
    });

    it('omits a row whose copy is absent rather than rendering a blank label', () => {
        expect(q(mount(about({ howToRead: '' })), '[data-row="how"]')).toBeNull();
    });
});
