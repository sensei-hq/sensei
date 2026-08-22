// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { tick } from 'svelte';

// Pretend Tauri is present so HealthState doesn't take the bypass path —
// these tests drive .apply() / .applyEvent() and inspect rendered UI.
(window as { __TAURI__?: unknown }).__TAURI__ = {};

import { mountComponent } from '$lib/test-mount.js';
import HealthView from './HealthView.svelte';
import { HealthState } from '$lib/health-state.svelte.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';
import type { HealthPayload, Remedy, ComponentStatus } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const remedy = (): Remedy => ({ message: 'msg', script: 'cmd', url: null });

const ok = (): HealthPayload => ({
  version: '0.2.14', uptimeSeconds: 0, platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null, installingVerb: 'installing', description: '' },
  components: COMPONENT_ORDER.map((id) => ({ id, label: String(id), note: null, status: 'ready' as const, version: '1.0', detail: null, installingVerb: 'installing', description: '' })),
  status: 'ok', remedy: null,
});

const needsAction = (): HealthPayload => ({
  ...ok(),
  packageManager: { ...ok().packageManager, status: 'failed' },
  components: COMPONENT_ORDER.map((id) => ({ id, label: String(id), note: null, status: 'failed' as const, version: null, detail: 'blocked', installingVerb: 'installing', description: '' })),
  status: 'needs-action', remedy: remedy(),
});

describe('HealthView', () => {
  it('mounts core sub-components', () => {
    const state = new HealthState(needsAction());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    // Both assertions used to key off bare tag names, which stopped meaning what
    // their comments claimed: the `<section>` was KanjiHeader's root, and after it
    // folded into PageHeader (a <header>) the check passed only because Remedy also
    // renders a <section>. Three components in this tree render a <header>, so that
    // one was ambiguous too. Keyed on the components themselves now.
    expect(m.container.querySelector('[data-component="page-header"]')).not.toBeNull();
    expect(m.container.querySelectorAll('header').length).toBeGreaterThanOrEqual(2); // Header + PageHeader
    // GateRow rows rendered for each gate
    expect(m.container.querySelectorAll('[data-component="gate-row"]').length).toBe(6);
  });

  it('does NOT render Remedy when status is not needs-action', () => {
    const state = new HealthState(ok());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).toBeNull();
  });

  it('renders Remedy when status is needs-action', () => {
    const state = new HealthState(needsAction());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).not.toBeNull();
  });

  it('renders the foundation note only while checking (not ok, resolving, or needs-action)', () => {
    // default HealthState status is 'checking'
    const checking = new HealthState();
    const m1 = mountComponent(HealthView, { state: checking });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('[data-component="foundation-note"]')).not.toBeNull();
    expect(m1.container.querySelector('[data-component="progress-card"]')).toBeNull();

    const okM = mountComponent(HealthView, { state: new HealthState(ok()) });
    cleanup.push(okM.destroy);
    expect(okM.container.querySelector('[data-component="foundation-note"]')).toBeNull();

    const naM = mountComponent(HealthView, { state: new HealthState(needsAction()) });
    cleanup.push(naM.destroy);
    expect(naM.container.querySelector('[data-component="foundation-note"]')).toBeNull();
  });

  it('renders the progress card (not the foundation note) while resolving', () => {
    const resolving: HealthPayload = {
      ...ok(),
      status: 'resolving',
      remedy: null,
      components: COMPONENT_ORDER.map((id, i) => {
        const status: ComponentStatus = i < 2 ? 'ready' : i === 2 ? 'installing' : 'pending';
        return { id, label: String(id), note: null, status, version: null, detail: null, installingVerb: 'installing', description: '' };
      }),
    };
    const m = mountComponent(HealthView, { state: new HealthState(resolving) });
    cleanup.push(m.destroy);
    const card = m.container.querySelector('[data-component="progress-card"]') as HTMLElement;
    expect(card).not.toBeNull();
    expect(card.textContent).toContain('ready'); // "N of M ready"
    expect(m.container.querySelector('[data-component="foundation-note"]')).toBeNull();
  });

  // When status=ok the right column stays visible (all six gates ready); the
  // +page.svelte auto-navigates via goto() so a separate Continue button is
  // not needed. The watermark logo fills the empty space in the left column.
  it('does NOT render Continue button when status=ok (auto-navigate handles it)', () => {
    const okState = new HealthState(ok());
    const m1 = mountComponent(HealthView, { state: okState });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('button[data-action="continue"]')).toBeNull();
  });

  it('does NOT render Continue button when status=needs-action', () => {
    const naState = new HealthState(needsAction());
    const m2 = mountComponent(HealthView, { state: naState });
    cleanup.push(m2.destroy);
    expect(m2.container.querySelector('button[data-action="continue"]')).toBeNull();
  });

  it('right column stays visible with all-green ledger when state.status flips to ok', async () => {
    const state = new HealthState(needsAction());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).not.toBeNull();
    expect(m.container.querySelectorAll('[data-component="gate-row"]').length).toBe(6);
    // Watermark only renders in ok state
    expect(m.container.querySelector('.watermark')).toBeNull();

    state.apply(ok());
    await tick();
    expect(m.container.querySelector('pre')).toBeNull();
    // Right column persists in ok state — user sees the all-green ledger
    expect(m.container.querySelectorAll('[data-component="gate-row"]').length).toBe(6);
    // Watermark fills the empty space in the left column
    expect(m.container.querySelector('.watermark')).not.toBeNull();
  });
});
