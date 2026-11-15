import { describe, it, expect, vi } from 'vitest';
import { MetricsHost } from './metrics-host.svelte.js';
import type { MetricsApi } from './metrics-state.svelte.js';
import type { ActionItem } from './action-items.js';

// Review r1 M3: the action items' state follows the workspace the window shows,
// keyed as `metrics` is (by the id's value). A cached one would post a snooze to
// the first workspace's URL, settle the first workspace's list, or show its
// per-item flows on another workspace's items with the same ids.

const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const tick = async () => {
    for (let i = 0; i < 5; i++) await Promise.resolve();
};

const SNOOZE: ActionItem = {
    id: 'r1', title: 'a recommendation', why: '', impact: '', urgency: 'none', urgencyLabel: '',
    action: { kind: 'snooze', label: 'Snooze 7 days', days: 7 },
};

function setup() {
    const api = {
        tryGetWorkspaceRecommendations: vi.fn(() =>
            ok([{ id: 'r1', title: 'one', status: 'pending' }, { id: 'r2', title: 'two', status: 'pending' }])),
        trySnoozeWorkspaceRecommendation: vi.fn(() => ok({ ok: true, snoozed_until: '2026-11-10T09:00:00+00:00' })),
        tryPreviewRecommendation: vi.fn(),
        tryMaterializeRecommendationWithReason: vi.fn(),
    } as unknown as MetricsApi & Record<string, ReturnType<typeof vi.fn>>;
    let id = 'w1';
    const host = new MetricsHost(() => id, () => api, vi.fn(() => Promise.resolve()));
    return { api, host, goTo: (next: string) => (id = next) };
}

describe('MetricsHost.actions', () => {
    it('one state per workspace: kept while the workspace stays, by the id’s value', () => {
        const { host, goTo } = setup();
        const first = host.actions;
        expect(host.actions).toBe(first);
        goTo('w1'); // an equal id from a new navigation is the same workspace
        expect(host.actions).toBe(first);
    });

    it('moving to another workspace acts, settles and keeps flows there', async () => {
        const { api, host, goTo } = setup();
        const onW1 = host.actions;
        goTo('w2');
        const onW2 = host.actions;
        expect(onW2).not.toBe(onW1);

        host.metrics.recommendations; // the w2 read
        await tick();
        await onW2.act(SNOOZE);

        expect(api.trySnoozeWorkspaceRecommendation).toHaveBeenCalledWith('w2', 'r1', 7);
        expect(host.metrics.workspaceId).toBe('w2');
        expect(host.metrics.recommendations.data?.map((r) => r.id)).toEqual(['r2']);
        expect(onW1.flow('r1')).toEqual({ status: 'idle' });
    });
});
