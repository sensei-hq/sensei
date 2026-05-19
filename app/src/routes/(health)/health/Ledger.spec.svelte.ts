// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Ledger from './Ledger.svelte';
import type { Component, ComponentStatus } from '$lib/health-types.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// Per-component verb mirroring the Rust DependencySpec so tests can seed
// the wire value without going through the transport.
const INSTALLING_VERBS: Record<Component['id'], string> = {
  postgres: 'starting',
  ollama:   'starting',
  sensei:   'installing',
  database: 'setting up',
  daemon:   'starting',
  homebrew: 'installing',
  winget:   'installing',
};

const row = (id: Component['id'], status: ComponentStatus, detail: string | null = null): Component => ({
  id, label: String(id), note: null, status, version: null, detail,
  installingVerb: INSTALLING_VERBS[id],
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

  // For non-installing statuses the badge mirrors the status word
  // verbatim. The `installing` status maps to a per-component verb
  // (service-style deps say "starting", database says "setting up",
  // sensei stays "installing") — covered by a separate test below.
  it.each(
    (['pending', 'checking', 'ready', 'failed'] as ComponentStatus[]).map(
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

  it.each([
    ['postgres', 'starting'],
    ['ollama',   'starting'],
    ['daemon',   'starting'],
    ['database', 'setting up'],
    ['sensei',   'installing'],
  ] as const)('renders %s installing badge as "%s"', (id, expectedVerb) => {
    const cs = allReady();
    const idx = COMPONENT_ORDER.indexOf(id);
    cs[idx] = row(id, 'installing');
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const badge = m.container.querySelector(`[data-row="${id}"] [data-badge]`);
    expect(badge?.textContent?.trim().toLowerCase()).toBe(expectedVerb);
  });

  it('failed row shows detail text', () => {
    const cs = allReady();
    cs[1] = row('ollama', 'failed', 'port 11434 in use');
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const detail = m.container.querySelector('[data-row="ollama"] [data-detail]');
    expect(detail?.textContent).toContain('port 11434 in use');
  });

  it('failure detail row is user-selectable (carries select-text class)', () => {
    const cs = allReady();
    cs[1] = row('ollama', 'failed', 'port 11434 in use');
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const detail = m.container.querySelector('[data-row="ollama"] [data-detail]');
    expect(detail?.className).toContain('select-text');
  });

  it('throws when components.length is not 5', () => {
    expect(() =>
      mountComponent(Ledger, { components: allReady().slice(0, 4) })
    ).toThrow(/expected 5 components/);
  });

  it('renders the row note when non-null', () => {
    const cs = allReady();
    cs[2] = { ...cs[2], note: 'cli · mcp · daemon' };
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const row = m.container.querySelector('[data-row="sensei"]');
    expect(row?.textContent).toContain('cli · mcp · daemon');
  });

  it('renders the row version (mono badge) when non-null', () => {
    const cs = allReady();
    cs[0] = { ...cs[0], version: '17.2' };
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const badge = m.container.querySelector('[data-row="postgres"] [data-version]');
    expect(badge?.textContent).toBe('17.2');
  });

  it('omits the version badge when null', () => {
    const cs = allReady(); // factory sets version: null
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-row="postgres"] [data-version]')).toBeNull();
  });
});
