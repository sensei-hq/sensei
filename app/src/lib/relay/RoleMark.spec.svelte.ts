// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import type { ComponentProps } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { KANJI_ICONS } from '$lib/glyph-vocabulary';
import { roleMarkClass } from './primitives.js';
import RoleMarkHarness from './RoleMark.harness.svelte';

// RoleMark (COMPONENTS.md §3a): a lane's 24px role circle, or a crew member's
// 30px status circle. Fixtures: both roles at 24, and every status at 30, in
// kanji and icon glyph mode. Theme is not a snapshot dimension (the mark draws
// tokens; the DOM is identical), and neither is layout — the size is a prop.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

type HarnessProps = ComponentProps<typeof RoleMarkHarness>;

function mount(props: HarnessProps) {
  const m = mountComponent(RoleMarkHarness, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-role-mark]') as HTMLElement;
  const glyph = root.querySelector('[data-lane-role]') as HTMLElement;
  return { ...m, root, glyph };
}

const laneCoord = { role: 'coordinator', size: 24 } as const;
const laneContrib = { role: 'contributor', size: 24 } as const;
const crewActive = { role: 'contributor', status: 'active', size: 30 } as const;
const crewIdle = { role: 'contributor', status: 'idle', size: 30 } as const;
const crewLost = { role: 'contributor', status: 'not_responding', size: 30 } as const;

describe('RoleMark', () => {
  it('lane (24): the coordinator is accent-tinted, a contributor sits on paper', () => {
    const coord = mount(laneCoord);
    expect(coord.root.dataset.size).toBe('24');
    expect(coord.root.className).toBe(roleMarkClass('coordinator', 'active', 24).frame);
    expect(coord.root.getAttribute('title')).toBe('Coordinator');
    expect(coord.glyph.textContent).toBe('指');

    const contrib = mount(laneContrib);
    expect(contrib.root.className).toContain('bg-paper');
    expect(contrib.root.getAttribute('title')).toBe('Contributor');
    expect(contrib.glyph.textContent).toBe('手');
  });

  it('crew (30): the circle and glyph carry the status tone, and the title says it', () => {
    const active = mount(crewActive);
    expect(active.root.dataset.size).toBe('30');
    expect(active.root.dataset.status).toBe('active');
    expect(active.root.className).toBe(roleMarkClass('contributor', 'active', 30).frame);
    expect(active.glyph.className).toContain('text-success');
    expect(active.root.getAttribute('title')).toBe('Contributor · Active');

    const idle = mount(crewIdle);
    expect(idle.glyph.className).toContain('text-ink-mute');
    expect(idle.root.getAttribute('title')).toBe('Contributor · Idle');

    const lost = mount(crewLost);
    expect(lost.root.dataset.status).toBe('not_responding');
    expect(lost.glyph.className).toContain('text-warning');
    expect(lost.root.getAttribute('title')).toBe('Contributor · Not responding');
  });

  it('draws the vocabulary icon in icon mode, the kanji in kanji mode', () => {
    const kanji = mount(laneCoord);
    expect(kanji.glyph.className).toContain('font-kanji');
    expect(kanji.glyph.className).not.toMatch(/\bi-/);

    const icon = mount({ ...laneCoord, glyphs: 'icon' });
    expect(icon.glyph.className).toContain(KANJI_ICONS['指']);
    expect(icon.glyph.textContent).toBe('');
    expect(icon.root.getAttribute('title')).toBe('Coordinator');

    const hand = mount({ ...laneContrib, glyphs: 'icon' });
    expect(hand.glyph.className).toContain(KANJI_ICONS['手']);
  });

  it('hides the glyph from the reader and keeps the title on the circle', () => {
    const { root, glyph } = mount(crewLost);
    expect(glyph.getAttribute('aria-hidden')).toBe('true');
    expect(root.getAttribute('aria-hidden')).toBeNull();
    expect(root.getAttribute('title')).toBe('Contributor · Not responding');
  });

  it('renders the same markup in dark mode — the tokens flip, not the DOM', () => {
    const light = mount(crewActive).root.outerHTML;
    document.documentElement.dataset.mode = 'dark';
    try {
      expect(mount(crewActive).root.outerHTML).toBe(light);
    } finally {
      delete document.documentElement.dataset.mode;
    }
  });

  it('pins every fixture in kanji mode', () => {
    const all = [laneCoord, laneContrib, crewActive, crewIdle, crewLost]
      .map((props) => mount(props).root.outerHTML)
      .join('\n');
    expect(all).toMatchSnapshot();
  });

  it('pins the icon-mode marks', () => {
    const all = [laneCoord, crewLost]
      .map((props) => mount({ ...props, glyphs: 'icon' }).root.outerHTML)
      .join('\n');
    expect(all).toMatchSnapshot();
  });
});
