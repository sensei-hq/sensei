// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import MemoryChipHarness from './MemoryChip.harness.svelte';
import type { ChipTone } from './memories-state.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

function mount(tone: ChipTone, label: string) {
    const m = mountComponent(MemoryChipHarness, { tone, label });
    cleanup.push(m.destroy);
    return m.container.querySelector('[data-component="memory-chip"]') as HTMLElement;
}

describe('MemoryChip', () => {
    it('renders the label text', () => {
        const el = mount('success', 'generalised: yes');
        expect(el.textContent?.trim()).toBe('generalised: yes');
    });

    it('reflects the tone on the data attribute', () => {
        expect(mount('success', 'x').getAttribute('data-tone')).toBe('success');
        expect(mount('neutral', 'x').getAttribute('data-tone')).toBe('neutral');
        expect(mount('accent', 'x').getAttribute('data-tone')).toBe('accent');
    });

    it('uses the success token pair for the success tone', () => {
        const el = mount('success', 'generalised: yes');
        expect(el.className).toContain('bg-success-soft');
        expect(el.className).toContain('text-success');
    });

    it('uses a muted paper/ink pair for the neutral tone', () => {
        const el = mount('neutral', 'generalised: no');
        expect(el.className).toContain('bg-paper-mute');
        expect(el.className).toContain('text-ink-mute');
    });

    it('uses the accent token pair for the accent tone', () => {
        const el = mount('accent', 'ready');
        expect(el.className).toContain('bg-accent-soft');
        expect(el.className).toContain('text-accent');
    });
});
