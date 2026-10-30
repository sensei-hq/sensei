// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import DiagramHeader from './DiagramHeader.svelte';
import { DiagramHeaderState } from './diagram-header.svelte.js';
import { DIAGRAM_VIEWS } from './diagrams-nav.js';
import { mountComponent, type MountResult } from '$lib/test-mount.js';

const store = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => void store.set(k, v),
  removeItem: (k: string) => void store.delete(k),
  clear: () => store.clear(),
});

let m: MountResult | null = null;
beforeEach(() => store.clear());
afterEach(() => { m?.destroy(); m = null; });

const STATS = [
  { label: 'Modules', value: '189', tone: 'ink' as const },
  { label: 'Symbols', value: '36,823', tone: 'ink' as const },
  { label: 'Cycles', value: '4', tone: 'warning' as const, lens: 'cycles' as const },
];

describe('DiagramHeader', () => {
  it('each stat is a card, its number over its name — modules and repos among them, no workspace tile', () => {
    m = mountComponent(DiagramHeader, {
      view: DIAGRAM_VIEWS[0], about: new DiagramHeaderState(), stats: STATS,
      workspace: { name: 'sensei', repos: 3 }, hrefFor: (l: string) => `/d/${l}`,
    });
    const cards = [...m.container.querySelectorAll('[data-testid^="diagram-stat-"]')] as HTMLElement[];
    expect(cards.map((c) => c.textContent?.replace(/\s+/g, ' ').trim())).toEqual(['189 Modules', '3 Repos', '36,823 Symbols', '4 Cycles']);
    expect(m.container.textContent).not.toContain('Workspace · sensei');
    expect(m.container.querySelector('[data-testid="diagram-stat-Cycles"] a')?.getAttribute('href')).toBe('/d/cycles');
  });
});
