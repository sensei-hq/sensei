// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { KANJI_ICONS } from '@sensei/glyphs';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './Family.harness.svelte';

// The shared family beside Notice (packages/ui): EmptyState for an honest empty,
// Banner for a strip, Loading for the wait. One set for sensei and the dōjō.

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });
function mount(props: { part: 'empty' | 'banner' | 'loading'; [k: string]: unknown }) {
  const m = mountComponent(Harness, props);
  cleanup.push(m.destroy);
  return m;
}
const q = (m: { container: HTMLElement }, sel: string) => m.container.querySelector(sel) as HTMLElement | null;

describe('EmptyState', () => {
  it('a mark, a title and a description — and status, never alert: empty is not a failure', () => {
    const m = mount({ part: 'empty', kanji: '静', title: 'Nothing needs you.', description: 'The queue is clear.' });
    const root = q(m, '[data-component="empty-state"]')!;
    expect(root.getAttribute('role')).toBe('status');
    expect(root.hasAttribute('data-empty')).toBe(true);
    expect(q(m, '[data-part="mark"]')!.textContent).toBe('静');
    expect(q(m, '[data-part="title"]')!.textContent).toBe('Nothing needs you.');
    expect(q(m, '[data-part="description"]')!.textContent).toBe('The queue is clear.');
  });

  it('the description may be children', () => {
    const m = mount({ part: 'empty', title: 'Nothing here.', body: 'No session matches that view.' });
    expect(q(m, '[data-part="description"]')!.textContent).toBe('No session matches that view.');
  });

  it('follows the glyph mode: an icon in icon mode', () => {
    const m = mount({ part: 'empty', glyphs: 'icon', kanji: '空', title: 't' });
    const mark = q(m, '[data-part="mark"]')!;
    expect(mark.dataset.glyph).toBe('icon');
    expect(mark.className).toContain(KANJI_ICONS['空']);
  });

  it('its action is a rokkit Button with its name, or a link', () => {
    const onclick = vi.fn();
    const m = mount({ part: 'empty', title: 't', action: { label: 'Add a folder', onclick, name: 'add' } });
    (q(m, '[data-action="add"]') as HTMLButtonElement).click();
    expect(onclick).toHaveBeenCalledOnce();
    const link = mount({ part: 'empty', title: 't', action: { label: 'Open settings', href: '/settings' } });
    expect(q(link, '[data-action="primary"]')!.getAttribute('href')).toBe('/settings');
  });

  it('has a panel size (a card) and a screen size (centred, no card)', () => {
    expect(q(mount({ part: 'empty', title: 't' }), '[data-component="empty-state"]')!.dataset.size).toBe('panel');
    expect(q(mount({ part: 'empty', title: 't', size: 'screen' }), '[data-component="empty-state"]')!.dataset.size).toBe('screen');
  });
});

describe('Banner', () => {
  it.each([
    ['fault', '難', 'alert', 'bg-danger-soft'],
    ['refusal', '止', 'alert', 'bg-warning-soft'],
    ['action', '手', 'status', 'bg-accent-soft'],
    ['normal', '待', 'status', 'bg-paper-soft'],
  ] as const)('%s: %s, role=%s, on %s', (tone, kanji, role, bg) => {
    const m = mount({ part: 'banner', tone, title: 'Reconnecting to the database…' });
    const root = q(m, '[data-component="banner"]')!;
    expect(root.dataset.tone).toBe(tone);
    expect(root.getAttribute('role')).toBe(role);
    expect(root.className).toContain(bg);
    expect(q(m, '[data-part="mark"]')!.textContent).toBe(kanji);
  });

  it('a mark of its own, a detail or children, and an action', () => {
    const onclick = vi.fn();
    const m = mount({ part: 'banner', tone: 'normal', mark: '観', title: 'These are counts.', detail: 'Never transcripts.', action: { label: 'Retry', onclick, name: 'retry' } });
    expect(q(m, '[data-part="mark"]')!.textContent).toBe('観');
    expect(q(m, '[data-part="detail"]')!.textContent).toBe('Never transcripts.');
    (q(m, '[data-action="retry"]') as HTMLButtonElement).click();
    expect(onclick).toHaveBeenCalledOnce();
    expect(q(mount({ part: 'banner', title: 't', body: 'more' }), '[data-part="detail"]')!.textContent).toBe('more');
  });

  it('an unknown tone reads as a fault', () => {
    expect(q(mount({ part: 'banner', tone: 'surprise', title: 't' }), '[data-component="banner"]')!.dataset.tone).toBe('fault');
  });
});

describe('Loading', () => {
  it('skeleton rows, busy, with words for a screen reader', () => {
    const m = mount({ part: 'loading', rows: 3, label: 'Loading tickets…' });
    const root = q(m, '[data-component="loading"]')!;
    expect(root.getAttribute('aria-busy')).toBe('true');
    expect(root.querySelectorAll('[data-part="row"]').length).toBe(3);
    expect(root.textContent).toContain('Loading tickets…');
  });
});
