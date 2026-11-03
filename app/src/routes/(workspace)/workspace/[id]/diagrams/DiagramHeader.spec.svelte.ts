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
  { label: 'Modules', value: '189', tone: 'ink' as const, icon: 'i-solar-box-minimalistic-bold-duotone' },
  { label: 'Symbols', value: '36,823', tone: 'ink' as const },
  { label: 'Cycles', value: '4', tone: 'warning' as const, lens: 'cycles' as const },
];

describe('DiagramHeader', () => {
  it('each stat is a stat card — its icon and name on top, its number below — modules and repos among them, no workspace tile', () => {
    m = mountComponent(DiagramHeader, {
      view: DIAGRAM_VIEWS[0], about: new DiagramHeaderState(), stats: STATS,
      workspace: { name: 'sensei', repos: 3 }, hrefFor: (l: string) => `/d/${l}`,
    });
    const cards = [...m.container.querySelectorAll('[data-testid^="diagram-stat-"]')] as HTMLElement[];
    expect(cards.every((c) => c.matches('[data-stat-card]'))).toBe(true);
    const read = (c: HTMLElement) =>
      `${c.querySelector('[data-stat-card-label]')?.textContent} ${c.querySelector('[data-stat-card-value]')?.textContent}`;
    expect(cards.map(read)).toEqual(['Modules 189', 'Repos 3', 'Symbols 36,823', 'Cycles 4']);
    expect(m.container.querySelector('[data-testid="diagram-stat-Modules"] [data-stat-card-icon]')?.className).toBe('i-solar-box-minimalistic-bold-duotone');
    expect(m.container.querySelector('[data-testid="diagram-stat-Cycles"]')?.getAttribute('data-tone')).toBe('warning');
    expect(m.container.textContent).not.toContain('Workspace · sensei');
    expect(m.container.querySelector('[data-testid="diagram-stat-Cycles"] a[data-stat-card-value]')?.getAttribute('href')).toBe('/d/cycles');
  });

  it('shows the focus as a chip under the title, and its ✕ clears it', () => {
    const onclear = vi.fn();
    m = mountComponent(DiagramHeader, {
      view: DIAGRAM_VIEWS[0], about: new DiagramHeaderState(), stats: STATS,
      workspace: { name: 'sensei', repos: 3 }, hrefFor: (l: string) => `/d/${l}`,
      focus: { label: 'senseid/db', onclear },
    });
    const chip = m.container.querySelector('[data-testid="diagram-focus"]') as HTMLElement;
    expect(chip.textContent).toContain('senseid/db');
    (chip.querySelector('button[aria-label="Clear focus"]') as HTMLButtonElement).click();
    expect(onclear).toHaveBeenCalledOnce();
  });

  it('shows no chip when nothing is focused', () => {
    m = mountComponent(DiagramHeader, {
      view: DIAGRAM_VIEWS[0], about: new DiagramHeaderState(), stats: STATS,
      workspace: { name: 'sensei', repos: 3 }, hrefFor: (l: string) => `/d/${l}`,
    });
    expect(m.container.querySelector('[data-testid="diagram-focus"]')).toBeNull();
  });
});
