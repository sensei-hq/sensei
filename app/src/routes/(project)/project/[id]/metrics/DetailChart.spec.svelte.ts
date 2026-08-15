// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import DetailChart from './DetailChart.svelte';
import type { ChartPoint } from '$lib/metrics/metric-view.js';

// Verifies the @rokkit/chart migration renders at runtime: the detail chart
// draws an area + line + endpoint via Plot geoms, breaks the line at an absent
// period (#6), and keeps the "not enough history" empty state.

let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});

function mount(
    series: ChartPoint[],
    yDomain: [number, number] = [0, 10],
    over: Record<string, unknown> = {},
) {
    const m = mountComponent(DetailChart, {
        series,
        yDomain,
        format: (v: number) => String(v),
        color: 'accent',
        caption: 'history from 2025-01-01 — no earlier data for this signal',
        ...over,
    });
    cleanup.push(m.destroy);
    return m.container;
}

const q = (root: HTMLElement, sel: string) => root.querySelector(sel) as HTMLElement | null;
const linePath = (root: HTMLElement) =>
    q(root, '[data-component="detail-chart"] path[data-plot-element="line"]')?.getAttribute('d') ?? '';
const moveCount = (d: string) => (d.match(/M/g) ?? []).length;

describe('DetailChart', () => {
    it('shows the empty state with fewer than two readings', () => {
        const root = mount([{ date: '2025-01-01', value: 1 }]);
        expect(root.textContent).toContain('Not enough history to chart yet.');
        expect(q(root, 'svg')).toBeNull();
    });

    it('renders an area, line and clickable datapoints via Plot geoms + overlay', () => {
        const root = mount([
            { date: '2025-01-01', value: 1 },
            { date: '2025-01-02', value: 3 },
            { date: '2025-01-03', value: 2 },
        ]);
        expect(q(root, '[data-component="detail-chart"] path[data-plot-element="line"]')).not.toBeNull();
        expect(q(root, '[data-component="detail-chart"] path[data-plot-element="area"]')).not.toBeNull();
        // A dot per defined datapoint + a transparent click-target per slot.
        expect(root.querySelectorAll('[data-component="detail-chart"] circle[data-point-dot]').length).toBe(3);
        expect(root.querySelectorAll('[data-component="detail-chart"] rect[data-hit]').length).toBe(3);
    });

    it('highlights the selected datapoint and emits the slot index on click', () => {
        let picked: number | null = null;
        const root = mount(
            [
                { date: '2025-01-01', value: 1 },
                { date: '2025-01-02', value: 3 },
                { date: '2025-01-03', value: 2 },
            ],
            [0, 10],
            { selectedIndex: 2, onselect: (i: number) => (picked = i) },
        );
        // The selected point is marked (drives the accent highlight + vertical guide).
        expect(q(root, 'circle[data-point-dot="2"]')?.getAttribute('data-selected')).toBe('true');
        expect(q(root, '[data-guide]')).not.toBeNull();
        // Clicking a datapoint's hit-target emits its slot index.
        (q(root, 'rect[data-hit="0"]') as HTMLElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
        expect(picked).toBe(0);
    });

    it('renders bars + a moving-average trend for a count metric (kind=bar)', () => {
        const root = mount(
            [
                { date: '2025-01-01', value: 4 },
                { date: '2025-01-02', value: 2 },
                { date: '2025-01-03', value: 6 },
            ],
            [0, 10],
            { kind: 'bar', selectedIndex: 2 },
        );
        // Bars, not line dots.
        expect(root.querySelectorAll('[data-component="detail-chart"] rect[data-bar]').length).toBe(3);
        expect(q(root, 'circle[data-point-dot]')).toBeNull();
        expect(q(root, 'rect[data-bar="2"]')?.getAttribute('data-selected')).toBe('true');
        // The moving-average trend line + the per-slot hit-targets are still there.
        expect(q(root, '[data-component="detail-chart"] path[data-plot-element="line"]')).not.toBeNull();
        expect(root.querySelectorAll('[data-component="detail-chart"] rect[data-hit]').length).toBe(3);
    });

    it('breaks the line at an absent period (#6), not a connected segment', () => {
        const connected = mount([
            { date: '2025-01-01', value: 1 },
            { date: '2025-01-02', value: 2 },
            { date: '2025-01-03', value: 3 },
        ]);
        expect(moveCount(linePath(connected))).toBe(1);

        const gapped = mount([
            { date: '2025-01-01', value: 1 },
            { date: '2025-01-02', value: 2 },
            { date: '2025-01-03', value: null },
            { date: '2025-01-04', value: 4 },
            { date: '2025-01-05', value: 5 },
        ]);
        expect(moveCount(linePath(gapped))).toBe(2);
    });

    it('renders the horizon caption', () => {
        const root = mount([
            { date: '2025-01-01', value: 1 },
            { date: '2025-01-02', value: 2 },
        ]);
        expect(root.textContent).toContain('history from 2025-01-01');
    });
});
