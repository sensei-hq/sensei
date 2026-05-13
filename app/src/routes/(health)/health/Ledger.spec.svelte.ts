// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Ledger from './Ledger.svelte';
import type { Component, ComponentStatus } from '$lib/health-types.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const row = (id: Component['id'], status: ComponentStatus, detail: string | null = null): Component => ({
  id, label: String(id), note: null, status, version: null, detail,
});

const allReady = (): Component[] => COMPONENT_ORDER.map((id) => row(id, 'ready'));

describe('Ledger', () => {
  it('renders 5 rows in COMPONENT_ORDER order', () => {
    const m = mountComponent(Ledger, { components: allReady() });
    cleanup.push(m.destroy);
    const labels = Array.from(m.container.querySelectorAll('[data-row]'))
      .map((el) => el.getAttribute('data-row'));
    expect(labels).toEqual([...COMPONENT_ORDER]);
  });

  it.each(
    (['pending', 'checking', 'installing', 'ready', 'failed'] as ComponentStatus[]).map(
      (s) => [s] as const,
    ),
  )('renders the %s badge', (s) => {
    const cs = allReady();
    cs[0] = row('postgres', s);
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const badge = m.container.querySelector('[data-row="postgres"] [data-badge]');
    expect(badge?.textContent?.trim().toLowerCase()).toBe(s);
  });

  it('failed row shows detail text', () => {
    const cs = allReady();
    cs[1] = row('ollama', 'failed', 'port 11434 in use');
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const detail = m.container.querySelector('[data-row="ollama"] [data-detail]');
    expect(detail?.textContent).toContain('port 11434 in use');
  });

  it('throws when components.length is not 5', () => {
    expect(() =>
      mountComponent(Ledger, { components: allReady().slice(0, 4) })
    ).toThrow(/expected 5 components/);
  });
});
