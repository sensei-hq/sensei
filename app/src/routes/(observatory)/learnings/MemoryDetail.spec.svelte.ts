// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { memoryState } from '$lib/memoryState.svelte.js';
import { mockMemory, mockMemoryDetail } from '$lib/setup/mock-contracts.js';
import MemoryDetail from './MemoryDetail.svelte';

// Isolate the singleton state module from the daemon api + app-state import chain.
vi.mock('$lib/api.js', () => ({ senseiApi: (_port: number) => ({}) }));
vi.mock('$lib/appstate.svelte.js', () => ({ appState: { port: 7745 } }));

let cleanup: Array<() => void> = [];
beforeEach(() => { memoryState.detail = null; });
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; memoryState.detail = null; });

function mount() {
    const m = mountComponent(MemoryDetail, {});
    cleanup.push(m.destroy);
    return m;
}

describe('MemoryDetail usage strip', () => {
    it('renders the three 7-day telemetry values from the wire fields', () => {
        memoryState.detail = mockMemoryDetail({
            loaded_last_7d: 5, followed_last_7d: 3, skipped_last_7d: 1,
        });
        const m = mount();
        const strip = m.container.querySelector('[data-testid="usage-strip"]');
        expect(strip).toBeTruthy();
        const text = strip!.textContent ?? '';
        expect(text).toContain('loaded 5 times');
        expect(text).toContain('followed 3');
        expect(text).toContain('skipped 1');
        expect(text).toContain('in the last 7 days');
    });

    it('renders genuine zeros (0 · 0 · 0), never blank or hidden', () => {
        memoryState.detail = mockMemoryDetail(); // all three default to 0
        const m = mount();
        const strip = m.container.querySelector('[data-testid="usage-strip"]');
        expect(strip).toBeTruthy();
        const text = strip!.textContent ?? '';
        expect(text).toContain('loaded 0 times');
        expect(text).toContain('followed 0');
        expect(text).toContain('skipped 0');
    });

    it('keeps the lifetime applied/violated counters as a secondary line', () => {
        memoryState.detail = mockMemoryDetail({
            memory: mockMemory({ strength: 3, applied_count: 12, violated_count: 2 }),
            loaded_last_7d: 4, followed_last_7d: 4, skipped_last_7d: 0,
        });
        const m = mount();
        const lifetime = m.container.querySelector('[data-testid="usage-lifetime"]');
        expect(lifetime).toBeTruthy();
        const text = lifetime!.textContent ?? '';
        expect(text).toContain('applied 12×');
        expect(text).toContain('violated 2×');
        expect(text).toContain('all-time');
    });
});
