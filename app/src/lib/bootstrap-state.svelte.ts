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
    Object.fromEntries(GATES.map(g => [g.id, 'waiting' as GateStatus]))
  );

  subStatuses = $state<Record<string, GateStatus>>({
    cli: 'waiting', mcp: 'waiting', daemon: 'waiting',
  });

  /** Original check error per gate — populated from event detail when status is 'blocked'. */
  gateErrors = $state<Record<string, string>>({});

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
      const idMap: Record<string, string> = { postgresql: 'postgres', daemon: 'senseid' };
      const id = idMap[event.id] ?? event.id;
      this.statuses = { ...this.statuses, [id]: status };
      // Capture check error (detail) for blocked gates so the UI can display version mismatches
      if (status === 'blocked') {
        const detail = event.data.detail as string | null | undefined;
        if (detail) this.gateErrors = { ...this.gateErrors, [id]: detail };
      } else {
        const { [id]: _removed, ...rest } = this.gateErrors;
        this.gateErrors = rest;
      }
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
      error: this.gateErrors[g.id] ?? null,
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
      return s === 'missing' || s === 'blocked';
    });
  }

  get isChecking(): boolean {
    return Object.values(this.statuses).some(s => s === 'checking' || s === 'installing' || s === 'starting');
  }

  get missingPrereqGates() {
    return this.gates.filter(g => g.remedy === 'prereq' && (g.status === 'missing' || g.status === 'blocked'));
  }

  get needsPrereqInstall(): boolean {
    return this.missingPrereqGates.length > 0;
  }

  get visibleGates() {
    return this.needsPrereqInstall
      ? this.gates.filter(g => g.id === 'homebrew')
      : this.gates;
  }

  // ── Simplified view ───────────────────────────────────────────

  /** Gate IDs shown in the item ledger (homebrew is the hero card, not a ledger row). */
  static readonly LEDGER_GATES = ['postgres', 'ollama', 'sensei', 'database', 'senseid'] as const;

  /** Status of the homebrew gate (hero card). */
  get homebrewStatus(): GateStatus {
    return this.statuses['homebrew'] ?? 'waiting';
  }

  /**
   * Simplified three/four-state machine for the bootstrap-simple UI.
   *   detecting  — all gates still at initial `waiting` state
   *   auto-fixing — engine is running (checking | installing | starting)
   *   manual      — engine finished but some gates are still blocked/missing
   *   all-green   — every gate is ready
   */
  get simpleState(): 'detecting' | 'auto-fixing' | 'manual' | 'all-green' {
    if (this.allReady) return 'all-green';
    if (this.isChecking) return 'auto-fixing';
    const allWaiting = Object.values(this.statuses).every(s => s === 'waiting');
    if (allWaiting) return 'detecting';
    return 'manual';
  }

  /**
   * Index of the currently-active ledger row (0–4).
   * While auto-fixing: first gate in checking/installing/starting.
   * Otherwise: count of ready ledger gates (used as a progress cursor).
   */
  get activeSimpleItemIdx(): number {
    const gates = BootstrapState.LEDGER_GATES;
    for (let i = 0; i < gates.length; i++) {
      const s = this.statuses[gates[i]];
      if (s === 'checking' || s === 'installing' || s === 'starting') return i;
    }
    return gates.filter(id => this.statuses[id] === 'ready').length;
  }

  // ── Mutations (used by page for browser-mode simulation only) ──

  setGateStatus(id: string, status: GateStatus) {
    this.statuses = { ...this.statuses, [id]: status };
  }

  setSubStatus(id: string, status: GateStatus) {
    this.subStatuses = { ...this.subStatuses, [id]: status };
    const allSubReady = ['cli', 'mcp', 'daemon'].every(k => this.subStatuses[k] === 'ready');
    const anySubMissing = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'missing' || this.subStatuses[k] === 'blocked');
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
