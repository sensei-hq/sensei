/**
 * Bootstrap state — manages gate statuses for the bootstrap screen.
 * Matches the gate structure from docs/mockups/lib/bootstrap.jsx.
 */

import { GATES, type GateStatus, type GateDefinition } from './bootstrap-gates.js';

export interface PlatformInfo {
  platform: 'macos' | 'linux' | 'windows';
  package_manager: string;
  prereq_remedy: { title: string; command: string; url?: string };
  pkgmgr_remedy: { title: string; command: string; url?: string };
}

export interface BootstrapEvent {
  action: 'update' | 'set';
  entity: 'gate' | 'phase';
  id: string;
  data: Record<string, unknown>;
}

export class BootstrapState {
  statuses = $state<Record<string, GateStatus>>(
    Object.fromEntries(GATES.map(g => [g.id, 'pending' as GateStatus]))
  );

  subStatuses = $state<Record<string, GateStatus>>({
    cli: 'pending', mcp: 'pending', daemon: 'pending',
  });

  installing = $state(false);

  platformInfo = $state<PlatformInfo>({
    platform: 'macos',
    package_manager: 'Homebrew',
    prereq_remedy: { title: 'Install missing components', command: 'curl -fsSL https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile | brew bundle --file=-', url: 'https://github.com/sensei-hq/homebrew-tap' },
    pkgmgr_remedy: { title: 'Install Homebrew', command: '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"', url: 'https://brew.sh' },
  });

  // ── Event handler (single entry point for all events) ─────

  handleEvent(event: BootstrapEvent) {
    if (event.action === 'update' && event.entity === 'gate') {
      const status = event.data.status as GateStatus;
      // Map backend gate IDs to frontend gate IDs
      const id = event.id === 'postgresql' ? 'postgres' : event.id;
      this.statuses = { ...this.statuses, [id]: status };
    }
    if (event.action === 'set' && event.entity === 'phase') {
      if (event.data.complete) {
        this.installing = false;
      }
    }
  }

  // ── Derived ───────────────────────────────────────────────

  get gates() {
    return GATES.map(g => ({
      ...g,
      name: g.id === 'homebrew' ? this.platformInfo.package_manager : g.name,
      status: this.statuses[g.id] ?? 'pending' as GateStatus,
      sub: g.sub?.map(s => ({ ...s, status: this.subStatuses?.[s.id] ?? 'pending' as GateStatus })),
    }));
  }

  get readyCount(): number {
    return GATES.filter(g => this.statuses[g.id] === 'ready').length;
  }

  get totalCount(): number { return GATES.length; }

  get allReady(): boolean {
    return GATES.every(g => this.statuses[g.id] === 'ready');
  }

  get firstBlockedIdx(): number {
    return GATES.findIndex(g => {
      const s = this.statuses[g.id];
      return s === 'missing' || s === 'error';
    });
  }

  get isChecking(): boolean {
    return Object.values(this.statuses).some(s => s === 'checking' || s === 'starting');
  }

  get missingPrereqGates() {
    return this.gates.filter(g => g.remedy === 'prereq' && (g.status === 'missing' || g.status === 'error'));
  }

  get needsPrereqInstall(): boolean {
    return this.missingPrereqGates.length > 0;
  }

  get visibleGates() {
    return this.needsPrereqInstall
      ? this.gates.filter(g => g.id === 'homebrew')
      : this.gates;
  }

  // ── Mutations (used by page for browser-mode simulation only) ──

  setGateStatus(id: string, status: GateStatus) {
    this.statuses = { ...this.statuses, [id]: status };
  }

  setSubStatus(id: string, status: GateStatus) {
    this.subStatuses = { ...this.subStatuses, [id]: status };
    const allSubReady = ['cli', 'mcp', 'daemon'].every(k => this.subStatuses[k] === 'ready');
    const anySubMissing = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'missing' || this.subStatuses[k] === 'error');
    const anySubChecking = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'checking');
    if (allSubReady) this.setGateStatus('sensei', 'ready');
    else if (anySubMissing) this.setGateStatus('sensei', 'missing');
    else if (anySubChecking) this.setGateStatus('sensei', 'checking');
  }

  setPlatform(info: PlatformInfo) {
    this.platformInfo = info;
  }

  applyPreset(preset: Record<string, GateStatus>) {
    this.statuses = { ...this.statuses, ...preset };
  }
}

export const bootstrapState = new BootstrapState();
