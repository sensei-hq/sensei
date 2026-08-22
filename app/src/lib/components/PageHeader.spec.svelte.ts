// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import PageHeaderHarness from './PageHeader.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const root = (m: { container: HTMLElement }): HTMLElement =>
  m.container.querySelector('[data-component="page-header"]')!;

describe('PageHeader', () => {
  it('renders title', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'Sessions' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Sessions');
  });

  it('renders title inside an <h1>', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'Sessions' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('h1')?.textContent).toBe('Sessions');
  });

  it('renders eyebrow when provided', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X', eyebrow: 'Inbox' });
    cleanup.push(m.destroy);
    const eyebrow = root(m).querySelector('[data-component="eyebrow"]');
    expect(eyebrow?.textContent).toContain('Inbox');
  });

  it('omits eyebrow node when prop is absent', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('[data-component="eyebrow"]')).toBeNull();
  });

  it('renders kanji when provided', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'Sessions', kanji: '刻' });
    cleanup.push(m.destroy);
    const k = root(m).querySelector('[data-component="kanji"]');
    expect(k?.textContent).toBe('刻');
  });

  it('omits kanji node when prop is absent', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('[data-component="kanji"]')).toBeNull();
  });

  it('renders description paragraph when provided', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X', description: 'desc' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('p')?.textContent).toBe('desc');
  });

  it('omits description when prop is absent', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('p')).toBeNull();
  });

  it.each([
    // One `size` scale replaces the old `variant: h1|h2|h3` heading level. The
    // rendering is deliberately unchanged: lg/md keep the mockup's 40px signature
    // glyph (F5) and sm is the compact nested step. Asserted as scale stops, not
    // `text-[40px]` — `text-3xl` IS 40px (uno.config.js fontSize."3xl"), and a
    // test pinning the literal pins a §1.3 violation.
    ['lg', /\btext-2xl\b/, /\btext-3xl\b/],
    ['md', /\btext-xl\b/,  /\btext-3xl\b/],
    ['sm', /\btext-lg\b/,  /\btext-xl\b/],
  ] as const)('size %s scales title + glyph together', (size, titleRe, kanjiRe) => {
    const m = mountComponent(PageHeaderHarness, { title: 'X', kanji: '刻', size });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('h1')!.className).toMatch(titleRe);
    expect(root(m).querySelector('[data-component="kanji"]')!.className).toMatch(kanjiRe);
    expect(root(m).getAttribute('data-size')).toBe(size);
  });

  it('defaults to size md', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('h1')!.className).toMatch(/\btext-xl\b/);
    expect(root(m).getAttribute('data-size')).toBe('md');
  });

  it('scales the description with the size, not independently of it', () => {
    const big = mountComponent(PageHeaderHarness, { title: 'X', description: 'd', size: 'md' });
    cleanup.push(big.destroy);
    const small = mountComponent(PageHeaderHarness, { title: 'X', description: 'd', size: 'sm' });
    cleanup.push(small.destroy);
    const cls = (m: { container: HTMLElement }) =>
      root(m).querySelector('[data-component="page-header-description"]')!.className;
    expect(cls(big)).toMatch(/\btext-sm\b/);
    expect(cls(small)).toMatch(/\btext-xs\b/);
  });

  it('wraps the description by default and clamps it only when asked', () => {
    // A screen's own copy is short and should wrap in full; clamping is for
    // caller-supplied text that can run long enough to push content off-screen.
    const plain = mountComponent(PageHeaderHarness, { title: 'X', description: 'd' });
    cleanup.push(plain.destroy);
    expect(
      root(plain).querySelector('[data-component="page-header-description"]')!.className,
    ).not.toMatch(/line-clamp/);

    const clamped = mountComponent(PageHeaderHarness, {
      title: 'X',
      description: 'd',
      clampDescription: true,
    });
    cleanup.push(clamped.destroy);
    expect(
      root(clamped).querySelector('[data-component="page-header-description"]')!.className,
    ).toMatch(/\bline-clamp-2\b/);
  });

  it('renders a count beside the title, and omits the node when absent', () => {
    const withCount = mountComponent(PageHeaderHarness, { title: 'X', count: 12 });
    cleanup.push(withCount.destroy);
    expect(
      root(withCount).querySelector('[data-component="page-header-count"]')!.textContent,
    ).toBe('12');

    const none = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(none.destroy);
    expect(root(none).querySelector('[data-component="page-header-count"]')).toBeNull();
  });

  it('renders a count of 0 rather than treating it as absent', () => {
    // `count={0}` is a real tally ("no rows"), not a missing prop — a falsy check
    // here would silently hide it.
    const m = mountComponent(PageHeaderHarness, { title: 'X', count: 0 });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('[data-component="page-header-count"]')!.textContent).toBe('0');
  });

  it('takes an icon when there is no kanji, and prefers the kanji when both are set', () => {
    const iconOnly = mountComponent(PageHeaderHarness, {
      title: 'X',
      icon: 'i-solar-folder-linear',
    });
    cleanup.push(iconOnly.destroy);
    expect(root(iconOnly).querySelector('[data-component="page-header-icon"]')!.className).toContain(
      'i-solar-folder-linear',
    );
    expect(root(iconOnly).querySelector('[data-component="kanji"]')).toBeNull();

    const both = mountComponent(PageHeaderHarness, {
      title: 'X',
      kanji: '刻',
      icon: 'i-solar-folder-linear',
    });
    cleanup.push(both.destroy);
    expect(root(both).querySelector('[data-component="kanji"]')).not.toBeNull();
    expect(root(both).querySelector('[data-component="page-header-icon"]')).toBeNull();
  });

  it('drops its own padding when nested in a container that already pads', () => {
    const padded = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(padded.destroy);
    expect(root(padded).className).toMatch(/\bpx-6\b/);

    const flush = mountComponent(PageHeaderHarness, { title: 'X', padded: false });
    cleanup.push(flush.destroy);
    expect(flush.container.querySelector('[data-component="page-header"]')!.className).not.toMatch(
      /\bpx-6\b/,
    );
  });

  it('applies hairline border-bottom by default', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(m.destroy);
    expect(root(m).className).toMatch(/\bborder-b\b/);
  });

  it('omits border-bottom when bordered=false', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X', bordered: false });
    cleanup.push(m.destroy);
    expect(root(m).className).not.toMatch(/\bborder-b\b/);
  });

  it('renders right snippet content', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X', rightText: 'STAT' });
    cleanup.push(m.destroy);
    expect(root(m).textContent).toContain('STAT');
  });
});
