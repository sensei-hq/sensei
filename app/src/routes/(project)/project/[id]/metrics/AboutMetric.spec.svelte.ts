// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import AboutMetricHarness from './AboutMetric.harness.svelte';
import type { MetricAbout, TextSegment } from '$lib/metrics/metric-view.js';

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

function mount(a: MetricAbout, howToReadSegments: TextSegment[] = [], projectId = '') {
    const m = mountComponent(AboutMetricHarness, { about: a, howToReadSegments, projectId });
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

    it('renders a companion metric in how-to-read as a link to its detail', () => {
        const segs: TextSegment[] = [
            { text: 'read with ' },
            { text: 'rework ratio', key: 'rework_ratio' },
            { text: ' to catch deferred work' },
        ];
        const root = mount(about(), segs, 'proj-1');
        const link = q(root, '[data-row="how"] a[data-companion="rework_ratio"]');
        expect(link?.getAttribute('href')).toBe('/project/proj-1/metrics/rework_ratio');
        expect(link?.textContent).toBe('rework ratio');
        // The surrounding prose is preserved verbatim.
        expect(q(root, '[data-row="how"]')?.textContent).toBe('read with rework ratio to catch deferred work');
    });

    it('does not link when no projectId is available (plain text, no anchor)', () => {
        const segs: TextSegment[] = [{ text: 'rework ratio', key: 'rework_ratio' }];
        const root = mount(about(), segs, '');
        expect(q(root, '[data-row="how"] a')).toBeNull();
        expect(q(root, '[data-row="how"]')?.textContent).toBe('rework ratio');
    });

    // The mockups (03-ev.png, 04-ev.png) name these columns, and the labels are
    // the whole point: they turn a bare number into something interpretable.
    // Previously "What it tells you" / "How it's computed".
    it.each([
        ['What it measures'],
        ['How to read it'],
        ['How this number was calculated'],
    ])('labels its columns "%s" as the mockup names them', (label) => {
        const text = mount(about()).textContent ?? '';
        expect(text).toContain(label);
    });

    it('renders unconditionally — the meaning is content, not a popover', () => {
        // It used to sit behind an info toggle. There must be no control gating
        // it and no collapsed state to open.
        const root = mount(about());
        expect(q(root, '[data-component="metric-about"]')).not.toBeNull();
        expect(q(root, 'button')).toBeNull();
        expect(q(root, '[aria-expanded]')).toBeNull();
    });
});
