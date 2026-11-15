// The Metrics screen's SDLC settings panel: a workspace's hourly rate, minutes per
// story point and idle gap (project-level, never per person).
// Spec: docs/spec/screen/workspace-metrics-settings.md; daemon: tracker-sdlc-metrics S1.
import type { ApiResult } from '$lib/api.js';
import { errorFor } from '$lib/tracker-errors.js';
import type { MetricSettings } from '$lib/types.js';

export interface MetricSettingsApi {
  tryGetMetricSettings: (workspaceId: string) => Promise<ApiResult<MetricSettings>>;
  tryPutMetricSettings: (
    workspaceId: string,
    patch: Record<string, number | string | null>,
  ) => Promise<ApiResult<MetricSettings>>;
}

export type SettingKey = keyof MetricSettings;

/** The inputs, in order, with their words. */
export const SETTINGS: ReadonlyArray<{ key: SettingKey; label: string; help: string }> = [
  { key: 'hourly_rate', label: 'Hourly rate', help: 'For effort and rework cost. Project-level, never per person.' },
  { key: 'minutes_per_point', label: 'Minutes per story point', help: 'For expected time and its variance.' },
  { key: 'idle_threshold_minutes', label: 'Idle gap that ends a work slice (minutes)', help: 'Default 15.' },
];

const text = (v: number | null) => (v == null ? '' : String(v));

/** An input as the daemon should get it: empty is null, a number is a number,
 *  anything else is sent as typed so the daemon names it. */
function wire(s: string): number | string | null {
  const t = s.trim();
  if (t === '') return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : t;
}

type Failure = { status: number; message: string; body?: Record<string, unknown> };
const failureText = (e: Failure) => errorFor(e.status, e.body ?? { error: e.message }).text;

export class MetricSettingsState {
  #api: MetricSettingsApi;
  #workspaceId: string;

  view = $state<'loading' | 'error' | 'ready'>('loading');
  loadError = $state<string | null>(null);
  /** What the daemon last said, the baseline for "changed". */
  stored = $state<MetricSettings | null>(null);
  draft = $state<Record<SettingKey, string>>({ hourly_rate: '', minutes_per_point: '', idle_threshold_minutes: '' });
  /** The key a refused save named. */
  fieldError = $state<SettingKey | null>(null);
  alert = $state<string | null>(null);
  notice = $state<string | null>(null);
  saving = $state(false);

  constructor(api: MetricSettingsApi, workspaceId: string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  /** The keys whose input differs from what is stored, as the daemon should get them. */
  get changes(): Record<string, number | string | null> {
    const s = this.stored;
    if (!s) return {};
    return Object.fromEntries(
      SETTINGS.filter(({ key }) => this.draft[key].trim() !== text(s[key])).map(({ key }) => [key, wire(this.draft[key])]),
    );
  }

  get dirty(): boolean {
    return Object.keys(this.changes).length > 0;
  }

  #adopt(s: MetricSettings) {
    this.stored = s;
    this.draft = { hourly_rate: text(s.hourly_rate), minutes_per_point: text(s.minutes_per_point), idle_threshold_minutes: text(s.idle_threshold_minutes) };
  }

  async load() {
    this.view = 'loading';
    const res = await this.#api.tryGetMetricSettings(this.#workspaceId);
    if (!res.ok) {
      this.loadError = failureText(res.error);
      this.view = 'error';
      return;
    }
    this.loadError = null;
    this.#adopt(res.data);
    this.view = 'ready';
  }

  async save() {
    const patch = this.changes;
    if (Object.keys(patch).length === 0) return;
    this.saving = true;
    this.fieldError = null;
    this.alert = null;
    this.notice = null;
    const res = await this.#api.tryPutMetricSettings(this.#workspaceId, patch);
    this.saving = false;
    if (res.ok) {
      this.#adopt(res.data);
      this.notice = 'Saved.';
      return;
    }
    const key = 'body' in res.error ? res.error.body?.key : undefined;
    const named = SETTINGS.find((s) => s.key === key)?.key;
    if (res.error.status === 400 && named) this.fieldError = named;
    else this.alert = failureText(res.error);
  }
}
