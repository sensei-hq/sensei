/**
 * Bootstrap state — manages gate statuses for the bootstrap screen.
 * Matches the gate structure from docs/mockups/lib/bootstrap.jsx.
 */

import { GATES, type GateStatus, type GateDefinition } from './bootstrap-gates.js';

export class BootstrapState {
  statuses = $state<Record<string, GateStatus>>(
    Object.fromEntries(GATES.map(g => [g.id, 'pending' as GateStatus]))
  );

  subStatuses = $state<Record<string, GateStatus>>({
    cli: 'pending', mcp: 'pending', daemon: 'pending',
  });

  dbUrl = $state('postgresql://localhost:5432/sensei');

  // ── Derived ────────────────────────────────────────────────

  get gates() {
    if (!this.statuses) return [];
    return GATES.map(g => ({
      ...g,
      status: this.statuses[g.id] ?? 'pending' as GateStatus,
      sub: g.sub?.map(s => ({ ...s, status: this.subStatuses?.[s.id] ?? 'pending' as GateStatus })),
    }));
  }

  get readyCount(): number {
    return GATES.filter(g => this.statuses[g.id] === 'ready').length;
  }

  get totalCount(): number {
    return GATES.length;
  }

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

  // ── Mutations ──────────────────────────────────────────────

  setGateStatus(id: string, status: GateStatus) {
    this.statuses = { ...this.statuses, [id]: status };
  }

  setSubStatus(id: string, status: GateStatus) {
    this.subStatuses = { ...this.subStatuses, [id]: status };
    // Derive sensei gate status from sub-checks
    const allSubReady = ['cli', 'mcp', 'daemon'].every(k => this.subStatuses[k] === 'ready');
    const anySubMissing = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'missing' || this.subStatuses[k] === 'error');
    const anySubChecking = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'checking');
    if (allSubReady) this.setGateStatus('sensei', 'ready');
    else if (anySubMissing) this.setGateStatus('sensei', 'missing');
    else if (anySubChecking) this.setGateStatus('sensei', 'checking');
  }

  /** Apply a preset (for mock/test scenarios). */
  applyPreset(preset: Record<string, GateStatus>) {
    this.statuses = { ...this.statuses, ...preset };
  }

  /** Set all gates to checking. */
  startChecking() {
    const first = GATES[0].id;
    this.statuses = Object.fromEntries(
      GATES.map((g, i) => [g.id, i === 0 ? 'checking' as GateStatus : 'pending' as GateStatus])
    );
  }
}

export const bootstrapState = new BootstrapState();
