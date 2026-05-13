// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Hero from './Hero.svelte';
import type { Component, ComponentStatus, HealthStatus } from '$lib/health-types.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const pm = (overrides: Partial<Component> = {}): Component => ({
  id: 'homebrew', label: 'Homebrew', note: null,
  status: 'ready' as ComponentStatus, version: '4.2.0', detail: null, ...overrides,
});

const allReady = (): Component[] => COMPONENT_ORDER.map((id) => ({
  id, label: id, note: null, status: 'ready' as const, version: '1.0', detail: null,
}));

const oneInstalling = (idx: number): Component[] => {
  const cs = allReady();
  cs[idx] = { ...cs[idx], status: 'installing' };
  return cs;
};

describe('Hero', () => {
  it('renders the package manager label', () => {
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'ok' as HealthStatus, components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Homebrew');
  });

  it('shows "Enter" button only when status is ok', () => {
    const m1 = mountComponent(Hero, {
      packageManager: pm(), status: 'ok' as HealthStatus, components: allReady(),
    });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('button[data-action="enter"]')).not.toBeNull();

    const m2 = mountComponent(Hero, {
      packageManager: pm(), status: 'checking' as HealthStatus, components: allReady(),
    });
    cleanup.push(m2.destroy);
    expect(m2.container.querySelector('button[data-action="enter"]')).toBeNull();
  });

  it('Enter button calls onEnter', () => {
    const onEnter = vi.fn();
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'ok' as HealthStatus, components: allReady(), onEnter,
    });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('button[data-action="enter"]') as HTMLButtonElement;
    btn.click();
    expect(onEnter).toHaveBeenCalledTimes(1);
  });

  it('shows "Detected" copy when status=ok', () => {
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'ok' as HealthStatus, components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Detected');
  });

  it('shows installing copy with the active component label when status=resolving', () => {
    const cs = oneInstalling(2); // sensei
    cs[2].label = 'Sensei components';
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'resolving' as HealthStatus, components: cs,
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Installing');
    expect(m.container.textContent).toContain('Sensei components');
    expect(m.container.textContent).toContain('(3/5)');
  });

  it('falls back to ready-count progress when nothing is installing', () => {
    const cs = allReady();
    cs[3].status = 'pending'; cs[4].status = 'pending';
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'resolving' as HealthStatus, components: cs,
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/3\/5|\(3\/5\)/);
  });

  it('shows manual copy when status=needs-action', () => {
    const m = mountComponent(Hero, {
      packageManager: pm({ status: 'failed' }), status: 'needs-action' as HealthStatus, components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Couldn['']t finish/);
  });

  it('renders packageManager.note when non-null', () => {
    const m = mountComponent(Hero, {
      packageManager: pm({ note: 'which brew' }), status: 'ok' as HealthStatus, components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('which brew');
  });
});
