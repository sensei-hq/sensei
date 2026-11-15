// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import SignalRail from './SignalRail.svelte';
import type { SignalGroup, SignalVM } from '$lib/metrics/metric-view.js';

// The user (2026-11-02): the drill-down rail drew its own links, so the current
// signal's shade was a hand-picked bg-accent-soft that is wrong in dark mode. It
// is a rokkit List, which owns the current row's look in both themes.
let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});

const sig = (key: string, name: string, over: Partial<SignalVM> = {}) =>
    ({ key, name, trend: null, color: 'ink-faint', ...over }) as SignalVM;
const groups: SignalGroup[] = [
    {
        id: 'key',
        label: 'Key signals',
        signals: [
            sig('ftr', 'First-try rate', { trend: { label: '+4%' } as SignalVM['trend'], color: 'success' }),
            sig('throughput', 'Throughput'),
        ],
    },
    { id: 'usage', label: 'Usage', signals: [sig('tokens_per_day', 'Tokens per day')] },
];

function mount(selectedKey = 'throughput') {
    const m = mountComponent(SignalRail, {
        groups,
        selectedKey,
        workspaceId: 'w1',
        distribution: null,
        format: (v: number) => String(v),
    });
    cleanup.push(m.destroy);
    flushSync();
    return m.container;
}

describe('SignalRail', () => {
    it('is a rokkit List of links, one per signal, under its group', () => {
        const root = mount();
        const list = root.querySelector('[data-component="signal-rail"] [data-list]');
        expect(list).not.toBeNull();
        const links = [...root.querySelectorAll('[data-list] a[data-list-item]')] as HTMLAnchorElement[];
        expect(links.map((a) => a.getAttribute('href'))).toEqual([
            '/workspace/w1/metrics/ftr',
            '/workspace/w1/metrics/throughput',
            '/workspace/w1/metrics/tokens_per_day',
        ]);
        expect(root.textContent).toContain('Key signals');
        expect(root.textContent).toContain('Usage');
    });

    it('lets the List mark the current signal, and only that one', () => {
        const root = mount('throughput');
        const active = [...root.querySelectorAll('[data-list] a[data-list-item][data-active="true"]')];
        expect(active.map((a) => a.getAttribute('href'))).toEqual(['/workspace/w1/metrics/throughput']);
        // No hand-rolled current-row shade left to go wrong in dark mode.
        expect(root.querySelector('.bg-accent-soft')).toBeNull();
    });

    it("shows a moving signal's trend in its tone", () => {
        const root = mount();
        const trend = root.querySelector('a[href="/workspace/w1/metrics/ftr"] [data-signal-trend]');
        expect(trend?.textContent?.trim()).toBe('+4%');
        expect(trend?.className).toContain('text-success');
        expect(root.querySelector('a[href="/workspace/w1/metrics/throughput"] [data-signal-trend]')).toBeNull();
    });
});
