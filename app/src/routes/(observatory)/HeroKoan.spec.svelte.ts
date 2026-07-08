// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import HeroKoanHarness from './HeroKoan.harness.svelte';
import type { ObservatoryTodayHero } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const hero = (over: Partial<ObservatoryTodayHero> = {}): ObservatoryTodayHero => ({
  kanji: '聴',
  koan: 'The AI does not know your auth.',
  body: 'Three sessions corrected this week in lumen-auth.',
  impact: 'Projected FTR +14%',
  action: 'Review recommendation',
  source: 'from s-2891 · s-2889',
  noticed: 'noticed 2 days ago',
  ...over,
});

describe('HeroKoan', () => {
  it('renders the koan and body copy', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'mature' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('The AI does not know your auth.');
    expect(m.container.textContent).toContain('Three sessions corrected this week');
  });

  it('renders the action button routing to the default insights surface', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'mature' });
    cleanup.push(m.destroy);
    const action = m.container.querySelector('[data-hero-action]');
    expect(action).not.toBeNull();
    expect(action?.getAttribute('href')).toBe('/insights');
    expect(action?.textContent).toContain('Review recommendation');
  });

  it('honours a custom action route', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'mature', actionHref: '/settings/personas' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-action]')?.getAttribute('href')).toBe('/settings/personas');
  });

  it('omits the action button when the hero has no action (early state)', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero({ action: null }), mode: 'early' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-action]')).toBeNull();
  });

  it('renders the impact dot when impact is present, hides it when null', () => {
    const withImpact = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'mature' });
    cleanup.push(withImpact.destroy);
    expect(withImpact.container.querySelector('[data-hero-impact]')).not.toBeNull();

    const noImpact = mountComponent(HeroKoanHarness, { hero: hero({ impact: null }), mode: 'mature' });
    cleanup.push(noImpact.destroy);
    expect(noImpact.container.querySelector('[data-hero-impact]')).toBeNull();
  });

  it('renders the source · noticed provenance when known', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'mature' });
    cleanup.push(m.destroy);
    const src = m.container.querySelector('[data-hero-source]');
    expect(src?.textContent?.trim()).toBe('from s-2891 · s-2889 · noticed 2 days ago');
  });

  it('renders no provenance node — no dangling middot — when source and noticed are empty', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero({ source: '', noticed: '' }), mode: 'mature' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-source]')).toBeNull();
  });

  it('switches the vertical caption by mode', () => {
    const mature = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'mature' });
    cleanup.push(mature.destroy);
    expect(mature.container.textContent).toContain('sensei speaks');

    const early = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'early' });
    cleanup.push(early.destroy);
    expect(early.container.textContent).toContain('sensei is listening');
  });

  it('tags the root with the mode discriminator', () => {
    const m = mountComponent(HeroKoanHarness, { hero: hero(), mode: 'early' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="hero-koan"]')?.getAttribute('data-mode')).toBe('early');
  });
});
