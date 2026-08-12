// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import MetricCardHarness from './MetricCard.harness.svelte';
import type { MetricCardVM } from '$lib/metrics/metric-view.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});

function card(over: Partial<MetricCardVM> = {}): MetricCardVM {
    return {
        key: 'ftr',
        name: 'First-turn resolution (FTR)',
        value: '100%',
        sub: '2 / 2',
        trend: { label: '+29 pt', dir: 'up', tone: 'good' },
        howToRead: 'never read alone',
        purpose: 'north-star',
        ...over,
    };
}

function mount(c: MetricCardVM, series: (number | null)[] = []) {
    const m = mountComponent(MetricCardHarness, { card: c, series });
    cleanup.push(m.destroy);
    return m.container;
}

const q = (root: HTMLElement, sel: string) => root.querySelector(sel) as HTMLElement | null;

describe('MetricCard', () => {
    it('renders the name, value and sub', () => {
        const root = mount(card());
        expect(q(root, '[data-component="metric-value"]')?.textContent?.trim()).toBe('100%');
        expect(q(root, '[data-component="metric-sub"]')?.textContent?.trim()).toBe('2 / 2');
        expect(root.textContent).toContain('First-turn resolution (FTR)');
    });

    it('renders the trend chip and carries its tone on a data attribute', () => {
        const chip = q(mount(card()), '[data-component="metric-trend"]');
        expect(chip?.getAttribute('data-tone')).toBe('good');
        expect(chip?.className).toContain('text-success');
        expect(chip?.textContent).toContain('+29 pt');
    });

    it('marks a bad trend with the danger token', () => {
        const chip = q(
            mount(card({ trend: { label: '+5s', dir: 'up', tone: 'bad' } })),
            '[data-component="metric-trend"]',
        );
        expect(chip?.className).toContain('text-danger');
    });

    it('omits the trend chip when there is no trend', () => {
        expect(q(mount(card({ trend: null })), '[data-component="metric-trend"]')).toBeNull();
    });

    it('renders the em-dash value verbatim for an absent metric', () => {
        const root = mount(card({ value: '—', trend: null, sub: '' }));
        expect(q(root, '[data-component="metric-value"]')?.textContent?.trim()).toBe('—');
        expect(q(root, '[data-component="metric-sub"]')).toBeNull();
    });

    it('draws a sparkline only when given two or more points', () => {
        expect(q(mount(card(), [1]), '[data-component="metric-sparkline"]')).toBeNull();
        expect(q(mount(card(), [1, 2, 3]), '[data-component="metric-sparkline"]')).not.toBeNull();
    });

    // #6: an absent period (null) must break the line, not connect / zero-fill
    // across it. A break shows up as a second `moveTo` (M) in the SVG path.
    const linePath = (root: HTMLElement) =>
        q(root, '[data-component="metric-sparkline"] path[data-plot-element="line"]')?.getAttribute(
            'd',
        ) ?? '';
    const moveCount = (d: string) => (d.match(/M/g) ?? []).length;

    it('breaks the line at an absent period (a gap), not a connected segment', () => {
        expect(moveCount(linePath(mount(card(), [1, 2, 3, 4])))).toBe(1);
        expect(moveCount(linePath(mount(card(), [1, 2, null, 4, 5])))).toBe(2);
    });
});
