// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import type { ComponentProps } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Pill from './Pill.svelte';
import { PILL_TONES, pillClass } from './primitives.js';

// Pill (COMPONENTS.md §3a): a tone-tinted chip. Fixtures are the five tones.
// Theme is not a second snapshot: a primitive draws tokens, so light and dark
// render the same markup and the token layer flips the values (asserted below).
// Layout is not a dimension either — a pill never reflows; it stays on one line.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

type PillProps = ComponentProps<typeof Pill>;

function mount(props: PillProps) {
  const m = mountComponent(Pill, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-pill]') as HTMLElement;
  return { ...m, root };
}

const html = (props: PillProps) => mount(props).root.outerHTML;

describe('Pill', () => {
  it('renders its label with the tone hooks', () => {
    const { root } = mount({ label: 'sprint 47', tone: 'accent' });
    expect(root.textContent).toBe('sprint 47');
    expect(root.dataset.tone).toBe('accent');
    expect(root.className).toBe(pillClass('accent'));
  });

  it.each(PILL_TONES)('draws the %s tone', (tone) => {
    const { root } = mount({ label: tone, tone });
    expect(root.dataset.tone).toBe(tone);
    expect(root.className).toBe(pillClass(tone));
  });

  it('defaults to neutral', () => {
    const { root } = mount({ label: 'plain' });
    expect(root.dataset.tone).toBe('neutral');
    expect(root.className).toContain('bg-paper-mute');
  });

  it('renders the same markup in dark mode — the tokens flip, not the DOM', () => {
    const light = html({ label: 'sprint 47', tone: 'warning' });
    document.documentElement.dataset.mode = 'dark';
    try {
      expect(html({ label: 'sprint 47', tone: 'warning' })).toBe(light);
    } finally {
      delete document.documentElement.dataset.mode;
    }
  });

  it('pins every tone', () => {
    const all = PILL_TONES.map((tone) => html({ label: `tone ${tone}`, tone })).join('\n');
    expect(all).toMatchSnapshot();
  });
});
