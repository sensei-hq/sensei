/**
 * Bootstrap state store — reactive state with explicit transitions.
 *
 * Components read from this store. API layer feeds it data.
 * No fetch calls, no side effects — just state management.
 */

import type {
  BootstrapResult, ComponentStatus, ComponentState, HardwareInfo,
} from './bootstrap.js';

// ── State ────────────────────────────────────────────────────────────────────

let _components = $state<ComponentStatus[]>([]);
let _hardware = $state<HardwareInfo>({
  ram_gb: 0, cpu_cores: 0, gpu: null, metal_support: false, recommended_tier: 'minimum',
});
let _loading = $state(true);
let _actionInProgress = $state<string | null>(null);

// ── Derived ──────────────────────────────────────────────────────────────────

const _ready = $derived(
  _components.length > 0 &&
  _components.every(c => c.state.state === 'ready' || c.state.state === 'skipped')
);

// ── Public read-only accessors ───────────────────────────────────────────────

export function getComponents(): ComponentStatus[] { return _components; }
export function getHardware(): HardwareInfo { return _hardware; }
export function isLoading(): boolean { return _loading; }
export function isReady(): boolean { return _ready; }
export function getActionInProgress(): string | null { return _actionInProgress; }

// ── Transitions ──────────────────────────────────────────────────────────────

/** Apply a full bootstrap result (from API or Tauri). */
export function applyResult(result: BootstrapResult) {
  _components = result.components;
  _hardware = result.hardware;
  _loading = false;
}

/** Update a single component's status. */
export function updateComponent(name: string, status: ComponentStatus) {
  const idx = _components.findIndex(c => c.name === name);
  if (idx >= 0) {
    _components[idx] = status;
    _components = [..._components]; // trigger reactivity
  }
}

/** Mark a component as skipped. */
export function skipComponent(name: string) {
  updateComponent(name, {
    name,
    state: { state: 'skipped' },
    version: null,
    detail: null,
  });
}

/** Set loading state. */
export function setLoading(loading: boolean) {
  _loading = loading;
}

/** Set which component has an action in progress. */
export function setActionInProgress(name: string | null) {
  _actionInProgress = name;
}

/** Reset state to initial. */
export function reset() {
  _components = [];
  _hardware = { ram_gb: 0, cpu_cores: 0, gpu: null, metal_support: false, recommended_tier: 'minimum' };
  _loading = true;
  _actionInProgress = null;
}
