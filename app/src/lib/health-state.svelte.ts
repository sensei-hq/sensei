import type {
  HealthPayload, HealthEvent, Component, HealthStatus, Platform,
  PackageManagerId, ComponentId, Remedy,
} from './health-types.js';
import { COMPONENT_ORDER } from './health-types.js';

function emptyComponent(id: ComponentId): Component {
  return { id, label: id, note: null, status: 'pending', version: null, detail: null };
}

/** Deterministic default — status='checking' so the UI never flashes 'ok' pre-apply. */
export const emptyPayload: HealthPayload = {
  version: '',
  uptimeSeconds: 0,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'pending', version: null, detail: null },
  components: COMPONENT_ORDER.map(emptyComponent),
  status: 'checking',
  remedy: null,
};

export class HealthState {
  status         = $state<HealthStatus>('checking');
  version        = $state<string>('');
  latest         = $state<string | null>(null);
  platform       = $state<Platform>('macos');
  packageManager = $state<Component>(emptyPayload.packageManager);
  components     = $state<Component[]>(emptyPayload.components);
  remedy         = $state<Remedy | null>(null);

  constructor(seed: HealthPayload = emptyPayload) {
    this.apply(seed);
  }

  apply(p: HealthPayload): void {
    this.version        = p.version;
    this.platform       = p.platform;
    this.packageManager = p.packageManager;
    this.components     = p.components;
    this.remedy         = p.remedy;
    this.status         = p.status;
  }
}

export const healthState = new HealthState();
