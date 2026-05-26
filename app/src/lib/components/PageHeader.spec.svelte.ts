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
    ['h1', /\btext-2xl\b/, /\btext-3xl\b/],
    ['h2', /\btext-xl\b/,  /\btext-2xl\b/],
    ['h3', /\btext-lg\b/,  /\btext-xl\b/],
  ] as const)('variant %s sizes title + kanji per spec', (variant, titleRe, kanjiRe) => {
    const m = mountComponent(PageHeaderHarness, { title: 'X', kanji: '刻', variant });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('h1')!.className).toMatch(titleRe);
    expect(root(m).querySelector('[data-component="kanji"]')!.className).toMatch(kanjiRe);
  });

  it('defaults to variant h2', () => {
    const m = mountComponent(PageHeaderHarness, { title: 'X' });
    cleanup.push(m.destroy);
    expect(root(m).querySelector('h1')!.className).toMatch(/\btext-xl\b/);
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
