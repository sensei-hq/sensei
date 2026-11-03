// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import ZonesLens from './ZonesLens.svelte';
import type { ZonePoint, Zones } from './types';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const pt = (component: string, zone: ZonePoint['zone'], instability: number, abstractness: number): ZonePoint => ({
  component, types: 2, abstractTypes: 0, ca: 3, ce: 1, abstractness, instability, distance: 0.6, zone,
});
const DATA: Zones = {
  meanDistance: 0.4, coverage: { points: 3, unplaceable: 0 },
  points: [pt('p/config', 'pain', 0.1, 0.05), pt('p/ports', 'useless', 0.9, 0.95), pt('p/fine', 'main', 0.5, 0.5)],
};

// The user (2026-11-02): Zones should focus on the zone of pain and uselessness —
// those points are drawn full and named; the rest stay as faint context.
describe('ZonesLens', () => {
  it('names the points in focus and fades the rest', () => {
    target = document.body.appendChild(document.createElement('div'));
    app = mount(ZonesLens, { target, props: { data: DATA, filter: 'issues', zoom: 1, onopen: vi.fn() } });
    flushSync();
    const labels = [...target.querySelectorAll('[data-zone-label]')].map((t) => t.textContent?.trim());
    expect(labels).toEqual(['config', 'ports']);
    const opacity = (id: string) => target.querySelector(`[data-testid="zone-${id}"]`)?.getAttribute('fill-opacity');
    expect([opacity('p/config'), opacity('p/ports'), opacity('p/fine')]).toEqual(['0.9', '0.9', '0.12']);
  });
});
