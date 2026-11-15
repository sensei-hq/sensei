// @vitest-environment jsdom
//
// The SDLC settings panel as a user meets it: labelled inputs with the daemon's
// values, a refused value marked on its own input, and no defaults on a failed read.
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import axe from 'axe-core';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './MetricSettingsPanel.harness.svelte';
import { MetricSettingsState, type MetricSettingsApi } from './metric-settings-state.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const api = (over: Partial<MetricSettingsApi> = {}): MetricSettingsApi => ({
  tryGetMetricSettings: async () => ({ ok: true, data: { hourly_rate: null, minutes_per_point: 90, idle_threshold_minutes: 15 } }),
  tryPutMetricSettings: async () => ({
    ok: false,
    error: { status: 400, message: 'invalid_setting', body: { error: 'invalid_setting', key: 'idle_threshold_minutes' } },
  }),
  ...over,
});

async function open(over: Partial<MetricSettingsApi> = {}) {
  const panel = new MetricSettingsState(api(over), 'w-1');
  await panel.load();
  const m = mountComponent(Harness, { panel });
  cleanup.push(m.destroy);
  flushSync();
  const q = <T extends Element = HTMLElement>(sel: string) => m.container.querySelector(sel) as T | null;
  return { panel, m, q };
}

describe('MetricSettingsPanel', () => {
  it("shows the daemon's values in labelled inputs; an unset rate is empty", async () => {
    const { q } = await open();
    expect(q<HTMLInputElement>('[data-setting="hourly_rate"]')!.value).toBe('');
    expect(q<HTMLInputElement>('[data-setting="minutes_per_point"]')!.value).toBe('90');
    expect(q('label[for="setting-idle_threshold_minutes"]')!.textContent).toContain('Idle gap');
    expect(q<HTMLButtonElement>('[data-action="save"]')!.disabled).toBe(true);
  });

  it('a refused value is marked on its own input and tied to it', async () => {
    const { q, panel } = await open();
    panel.draft.idle_threshold_minutes = '0';
    flushSync();
    q<HTMLButtonElement>('[data-action="save"]')!.click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    const input = q<HTMLInputElement>('[data-setting="idle_threshold_minutes"]')!;
    expect(q('[data-setting-error="idle_threshold_minutes"]')!.textContent).toBe('Check this value.');
    expect(input.getAttribute('aria-describedby')).toContain('setting-idle_threshold_minutes-error');
    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(q('[data-setting-error="hourly_rate"]')).toBe(null);
  });

  it('a failed read shows the error and Retry, and no inputs', async () => {
    const { q } = await open({ tryGetMetricSettings: async () => ({ ok: false, error: { status: 0, message: 'Network error' } }) });
    expect(q('[role="alert"]')!.textContent).toBe('The sensei daemon didn’t respond.');
    expect(q('[data-action="retry"]')).not.toBe(null);
    expect(q('[data-setting]')).toBe(null);
  });

  it('has no serious or critical axe violations', async () => {
    const { m } = await open();
    const result = await axe.run(m.container, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa'] }, rules: { 'color-contrast': { enabled: false } } });
    expect(result.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical')).toEqual([]);
  });
});
