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
import type { HealthPayload, Remedy } from '$lib/health-types.js';

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
    expect(m.container.querySelector('header')).not.toBeNull();          // Header
    expect(m.container.querySelector('section')).not.toBeNull();         // KanjiHeader renders <section>
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
