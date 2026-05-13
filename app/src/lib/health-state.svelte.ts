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
    // INV-1: discriminated union covers compile-time; this is the runtime boundary
    if (p.status === 'needs-action' && p.remedy === null)
      throw new Error('HealthState: needs-action requires a remedy');
    const pAsUntyped = p as unknown as { status: string; remedy: Remedy | null };
    if (pAsUntyped.remedy !== null && pAsUntyped.status !== 'needs-action')
      throw new Error(`HealthState: status=${pAsUntyped.status} must not carry a remedy`);

    // INV-2
    if (p.components.length !== COMPONENT_ORDER.length)
      throw new Error(`HealthState: expected ${COMPONENT_ORDER.length} components, got ${p.components.length}`);
    for (let i = 0; i < COMPONENT_ORDER.length; i++) {
      if (p.components[i].id !== COMPONENT_ORDER[i])
        throw new Error(`HealthState: components[${i}].id must be "${COMPONENT_ORDER[i]}", got "${p.components[i].id}"`);
    }

    // INV-3
    const expectedPm: PackageManagerId = p.platform === 'windows' ? 'winget' : 'homebrew';
    if (p.packageManager.id !== expectedPm)
      throw new Error(`HealthState: platform=${p.platform} expects packageManager.id="${expectedPm}", got "${p.packageManager.id}"`);

    this.version        = p.version;
    this.platform       = p.platform;
    this.packageManager = p.packageManager;
    this.components     = p.components;
    this.remedy         = p.remedy;
    this.status         = p.status;
  }

  applyEvent(e: HealthEvent): void {
    switch (e.kind) {
      case 'phase':     this.status = e.phase; return;
      case 'component': this.#patch(e.id, e.patch); return;
      case 'remedy':    return; // implemented in T7
      case 'report':    return; // implemented in T8
      default: {
        const _exhaustive: never = e;
        throw new Error(`HealthState: unknown event kind ${JSON.stringify(_exhaustive)}`);
      }
    }
  }

  #patch(id: ComponentId | PackageManagerId, patch: Partial<Component>): void {
    if (id === this.packageManager.id) {
      this.packageManager = { ...this.packageManager, ...patch };
      return;
    }
    const idx = this.components.findIndex((c) => c.id === id);
    if (idx < 0) {
      throw new Error(
        `HealthState: unknown component id "${id}" — not in [${this.packageManager.id}, ${COMPONENT_ORDER.join(', ')}]`
      );
    }
    const next = this.components.slice();
    next[idx] = { ...next[idx], ...patch };
    this.components = next;
  }
}

export const healthState = new HealthState();
