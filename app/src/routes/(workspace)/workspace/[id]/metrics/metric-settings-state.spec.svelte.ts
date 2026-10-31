// The SDLC settings panel's state (docs/spec/screen/workspace-metrics-settings.md).
//
// What must never happen here: defaults shown for a failed read, 0 sent for a
// cleared rate, or a save that sends keys nobody changed.
import { describe, it, expect } from 'vitest';
import { MetricSettingsState, type MetricSettingsApi } from './metric-settings-state.svelte.js';
import type { ApiResult } from '$lib/api.js';
import type { MetricSettings } from '$lib/types.js';

const STORED: MetricSettings = { hourly_rate: null, minutes_per_point: 90, idle_threshold_minutes: 15 };

function daemon(over: Partial<MetricSettingsApi> = {}) {
  const puts: Array<Partial<MetricSettings>> = [];
  const api: MetricSettingsApi = {
    tryGetMetricSettings: async () => ({ ok: true, data: STORED }),
    tryPutMetricSettings: async (_w, patch) => {
      puts.push(patch);
      return { ok: true, data: { ...STORED, ...patch } as MetricSettings };
    },
    ...over,
  };
  return { api, puts };
}

async function ready(over: Partial<MetricSettingsApi> = {}) {
  const d = daemon(over);
  const s = new MetricSettingsState(d.api, 'w-1');
  await s.load();
  return { s, ...d };
}

describe('MetricSettingsState', () => {
  it("shows the daemon's values, an unset rate empty and never 0", async () => {
    const { s } = await ready();
    expect(s.view).toBe('ready');
    expect(s.draft).toEqual({ hourly_rate: '', minutes_per_point: '90', idle_threshold_minutes: '15' });
    expect(s.dirty).toBe(false);
  });

  it('a failed read is an error, never the defaults', async () => {
    const failed: ApiResult<MetricSettings> = { ok: false, error: { status: 0, message: 'Network error' } };
    const { s } = await ready({ tryGetMetricSettings: async () => failed });
    expect(s.view).toBe('error');
    expect(s.loadError).toBe('The sensei daemon didn’t respond.');
  });

  it('saves only what changed, and a cleared rate as null', async () => {
    const { s, puts } = await ready();
    s.draft.minutes_per_point = '120';
    expect(s.dirty).toBe(true);
    await s.save();
    expect(puts).toEqual([{ minutes_per_point: 120 }]);
    expect(s.notice).toBe('Saved.');
    expect(s.dirty).toBe(false);

    const set = await ready({ tryGetMetricSettings: async () => ({ ok: true, data: { ...STORED, hourly_rate: 50 } }) });
    set.s.draft.hourly_rate = '';
    await set.s.save();
    expect(set.puts).toEqual([{ hourly_rate: null }]);
  });

  it('a value the daemon refuses marks its own input and keeps what was typed', async () => {
    const refused: ApiResult<MetricSettings> = {
      ok: false,
      error: { status: 400, message: 'invalid_setting', body: { error: 'invalid_setting', key: 'idle_threshold_minutes' } },
    };
    const { s, puts } = await ready({
      tryPutMetricSettings: async (_w, patch) => {
        puts.push(patch);
        return refused;
      },
    });
    s.draft.idle_threshold_minutes = 'abc';
    await s.save();
    expect(puts).toEqual([{ idle_threshold_minutes: 'abc' }]);
    expect(s.fieldError).toBe('idle_threshold_minutes');
    expect(s.alert).toBe(null);
    expect(s.draft.idle_threshold_minutes).toBe('abc');
  });

  it('a cleared idle gap is sent as null, and the daemon\'s refusal marks it (it is required)', async () => {
    const refused: ApiResult<MetricSettings> = {
      ok: false,
      error: { status: 400, message: 'invalid_setting', body: { error: 'invalid_setting', key: 'idle_threshold_minutes' } },
    };
    const { s, puts } = await ready({
      tryPutMetricSettings: async (_w, patch) => {
        puts.push(patch);
        return refused;
      },
    });
    s.draft.idle_threshold_minutes = '';
    await s.save();
    expect(puts).toEqual([{ idle_threshold_minutes: null }]);
    expect(s.fieldError).toBe('idle_threshold_minutes');
  });

  it('Retry reads again after a failed read', async () => {
    let calls = 0;
    const { s } = await ready({
      tryGetMetricSettings: async () =>
        ++calls === 1 ? { ok: false, error: { status: 0, message: 'Network error' } } : { ok: true, data: STORED },
    });
    expect(s.view).toBe('error');
    await s.load();
    expect(s.view).toBe('ready');
  });

  it('any other failure is the panel alert', async () => {
    const { s } = await ready({
      tryPutMetricSettings: async () => ({ ok: false, error: { status: 0, message: 'Network error' } }),
    });
    s.draft.minutes_per_point = '60';
    await s.save();
    expect(s.alert).toBe('The sensei daemon didn’t respond.');
    expect(s.fieldError).toBe(null);
  });
});
