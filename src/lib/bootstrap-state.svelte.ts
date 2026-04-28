/**
 * Bootstrap state — singleton class wrapping reactive state.
 */

import type {
  BootstrapResult, ComponentStatus, ComponentState, HardwareInfo,
} from './bootstrap.js';

class BootstrapState {
  components = $state<ComponentStatus[]>([]);
  hardware = $state<HardwareInfo>({
    ram_gb: 0, cpu_cores: 0, gpu: null, metal_support: false, recommended_tier: 'minimum',
  });
  loading = $state(true);
  actionInProgress = $state<string | null>(null);

  get ready() {
    return this.components.length > 0 &&
      this.components.every(c => c.state.state === 'ready' || c.state.state === 'skipped');
  }

  applyResult(result: BootstrapResult) {
    this.components = result.components;
    this.hardware = result.hardware;
    this.loading = false;
  }

  updateComponent(name: string, status: ComponentStatus) {
    const idx = this.components.findIndex(c => c.name === name);
    if (idx >= 0) {
      this.components[idx] = status;
      this.components = [...this.components];
    }
  }

  skipComponent(name: string) {
    this.updateComponent(name, {
      name, state: { state: 'skipped' }, version: null, detail: null,
    });
  }

  setLoading(val: boolean) { this.loading = val; }
  setAction(name: string | null) { this.actionInProgress = name; }
}

export const bootstrapState = new BootstrapState();
