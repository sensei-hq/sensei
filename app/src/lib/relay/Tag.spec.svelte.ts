// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Tag from './Tag.svelte';
import { tagClass } from './primitives.js';

// Tag (COMPONENTS.md §3a): the three tags a member card carries — the family
// (assistant name), the model (mono), and the host ("in Cursor"). Fixtures are
// the three kinds. Theme and layout are not snapshot dimensions: the tag draws
// tokens and never reflows (asserted below).

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function mount(props: Record<string, unknown>) {
  const m = mountComponent(Tag, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-tag]') as HTMLElement;
  return { ...m, root };
}

describe('Tag', () => {
  it('family: the assistant name on the mute surface', () => {
    const { root } = mount({ kind: 'family', label: 'Muse' });
    expect(root.dataset.tag).toBe('family');
    expect(root.textContent).toBe('Muse');
    expect(root.className).toBe(tagClass('family'));
    expect(root.getAttribute('title')).toBeNull();
  });

  it('model: the model id, mono', () => {
    const { root } = mount({ kind: 'model', label: 'claude-opus-5-5' });
    expect(root.dataset.tag).toBe('model');
    expect(root.textContent).toBe('claude-opus-5-5');
    expect(root.className).toContain('font-mono');
    expect(root.className).toBe(tagClass('model'));
  });

  it('host: reads "in <editor>" and explains the editor in its title', () => {
    const { root } = mount({ kind: 'host', editor: 'Cursor' });
    expect(root.dataset.tag).toBe('host');
    expect(root.textContent).toBe('in Cursor');
    expect(root.getAttribute('title')).toBe(
      'Cursor is the editor; the assistant inside it does the work.',
    );
    expect(root.className).toBe(tagClass('host'));
  });

  it('renders the same markup in dark mode — the tokens flip, not the DOM', () => {
    const light = mount({ kind: 'host', editor: 'Cursor' }).root.outerHTML;
    document.documentElement.dataset.mode = 'dark';
    try {
      expect(mount({ kind: 'host', editor: 'Cursor' }).root.outerHTML).toBe(light);
    } finally {
      delete document.documentElement.dataset.mode;
    }
  });

  it('pins all three kinds', () => {
    const all = [
      mount({ kind: 'family', label: 'Muse' }).root.outerHTML,
      mount({ kind: 'model', label: 'claude-opus-5-5' }).root.outerHTML,
      mount({ kind: 'host', editor: 'Cursor' }).root.outerHTML,
    ].join('\n');
    expect(all).toMatchSnapshot();
  });
});
