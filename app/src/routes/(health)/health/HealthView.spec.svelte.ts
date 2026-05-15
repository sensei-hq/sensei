// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
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
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null },
  components: COMPONENT_ORDER.map((id) => ({ id, label: String(id), note: null, status: 'ready' as const, version: '1.0', detail: null })),
  status: 'ok', remedy: null,
});

const needsAction = (): HealthPayload => ({
  ...ok(),
  packageManager: { ...ok().packageManager, status: 'failed' },
  components: COMPONENT_ORDER.map((id) => ({ id, label: String(id), note: null, status: 'failed' as const, version: null, detail: 'blocked' })),
  status: 'needs-action', remedy: remedy(),
});

describe('HealthView', () => {
  it('mounts all four sub-components', () => {
    const state = new HealthState(ok());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('header')).not.toBeNull();          // Header
    expect(m.container.querySelector('section')).not.toBeNull();         // Hero is first <section>
    expect(m.container.querySelector('ul')).not.toBeNull();              // Ledger
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

  it('renders "Continue →" footer button iff state.isOk', () => {
    const okState = new HealthState(ok());
    const m1 = mountComponent(HealthView, { state: okState });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('button[data-action="continue"]')).not.toBeNull();

    const naState = new HealthState(needsAction());
    const m2 = mountComponent(HealthView, { state: naState });
    cleanup.push(m2.destroy);
    expect(m2.container.querySelector('button[data-action="continue"]')).toBeNull();
  });

  it('Continue button calls onEnter', () => {
    const onEnter = vi.fn();
    const state = new HealthState(ok());
    const m = mountComponent(HealthView, { state, onEnter });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="continue"]') as HTMLButtonElement).click();
    expect(onEnter).toHaveBeenCalledTimes(1);
  });

  it('reactively toggles Remedy + Continue when state.status flips', async () => {
    const state = new HealthState(needsAction());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).not.toBeNull();
    expect(m.container.querySelector('button[data-action="continue"]')).toBeNull();

    state.apply(ok());
    await tick();
    expect(m.container.querySelector('pre')).toBeNull();
    expect(m.container.querySelector('button[data-action="continue"]')).not.toBeNull();
  });
});
