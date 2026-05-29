// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Hero from './Hero.svelte';
import type { Component, ComponentStatus, HealthStatus } from '$lib/health-types.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

/** Six gates in the new layout: PM (homebrew) + the 5 components. */
const sixReady = (): Component[] => [
  {
    id: 'homebrew', label: 'Homebrew', note: null,
    status: 'ready' as ComponentStatus, version: '4.2.0', detail: null,
    installingVerb: 'installing', description: 'pm',
  },
  ...COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'ready' as const, version: '1.0', detail: null,
    installingVerb: 'installing', description: id,
  })),
];

const withStatus = (idx: number, s: ComponentStatus): Component[] => {
  const cs = sixReady();
  cs[idx] = { ...cs[idx], status: s };
  return cs;
};

describe('Hero', () => {
  it('renders the "foundation" eyebrow', () => {
    const m = mountComponent(Hero, { status: 'ok' as HealthStatus, components: sixReady() });
    cleanup.push(m.destroy);
    expect(m.container.textContent?.toLowerCase()).toContain('foundation');
  });

  it('renders "holds" copy when status=ok', () => {
    const m = mountComponent(Hero, { status: 'ok' as HealthStatus, components: sixReady() });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('holds');
  });

  it('renders "checking" copy when status=checking', () => {
    const m = mountComponent(Hero, { status: 'checking' as HealthStatus, components: sixReady() });
    cleanup.push(m.destroy);
    expect(m.container.textContent?.toLowerCase()).toContain('checking');
  });

  it('renders progress count when status=resolving', () => {
    // 3 ready (homebrew/postgres/ollama), 1 installing (sensei), 2 pending (database/daemon)
    const cs = sixReady();
    cs[3] = { ...cs[3], status: 'installing' };
    cs[4] = { ...cs[4], status: 'pending' };
    cs[5] = { ...cs[5], status: 'pending' };
    const m = mountComponent(Hero, { status: 'resolving' as HealthStatus, components: cs });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/3\D+of\D+6/);
  });

  it('renders "needs your hand" copy when status=needs-action', () => {
    const cs = withStatus(0, 'failed');
    const m = mountComponent(Hero, { status: 'needs-action' as HealthStatus, components: cs });
    cleanup.push(m.destroy);
    expect(m.container.textContent?.toLowerCase()).toContain('needs your hand');
  });

  it('renders a spinner in the hero disc when status is busy', () => {
    const m = mountComponent(Hero, { status: 'resolving' as HealthStatus, components: withStatus(1, 'installing') });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-spinner]')).not.toBeNull();
  });

  it('does NOT render the spinner when status=ok', () => {
    const m = mountComponent(Hero, { status: 'ok' as HealthStatus, components: sixReady() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-spinner]')).toBeNull();
  });

  it('does NOT render an Enter button (moved to HealthView Continue)', () => {
    const m = mountComponent(Hero, { status: 'ok' as HealthStatus, components: sixReady() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('button[data-action="enter"]')).toBeNull();
  });
});
