// Model chains and the reasoning roles they serve — the one role write and the
// Settings · Inference state (audit 2026-10-28, item 13: the panel held its whole
// slice in the component and repeated the wizard's clear-then-set write).
import { failureReason, type SenseiApi } from '$lib/api.js';
import type { DaemonChain, SenseiRole } from './contracts.js';
import { ROLE_META, roleAssignments } from './role-meta.js';

export interface RoleChange {
  role: SenseiRole;
  prev: string | null;
  next: string | null;
}

/**
 * Move roles between chains. Two phases, because a chain can hold a role only
 * once the old holder has let it go (the daemon's unique-when-set index): clear
 * every old chain, then set every new one. Unchanged roles are skipped. Returns
 * what the daemon refused, in words; empty when everything stuck.
 */
export async function assignChainRoles(
  api: Pick<SenseiApi, 'setGatewayChainRole'>,
  changes: RoleChange[],
): Promise<string[]> {
  const moving = changes.filter((c) => (c.prev ?? null) !== (c.next ?? null));
  const refused: string[] = [];
  for (const c of moving) {
    if (!c.prev) continue;
    const r = await api.setGatewayChainRole(c.prev, null);
    if (!r.ok) refused.push(`clearing ${c.role}: ${failureReason(r.error)}`);
  }
  for (const c of moving) {
    if (!c.next) continue;
    const r = await api.setGatewayChainRole(c.next, c.role);
    if (!r.ok) refused.push(`assigning ${c.role}: ${failureReason(r.error)}`);
  }
  return refused;
}

type Api = Pick<
  SenseiApi,
  'tryListGatewayChains' | 'setGatewayChainRole' | 'moveGatewayChainModel' | 'removeGatewayChainModel'
>;
type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';
const ROLES: SenseiRole[] = ['inference', 'consolidation', 'embedding', 'voice'];
const byRole = <T>(v: T) => Object.fromEntries(ROLES.map((r) => [r, v])) as Record<SenseiRole, T>;
/** How long "saved" shows before the row settles. */
const SAVED_MS = 1500;

/** Settings · Inference: which chain serves each role, and its members' order. */
export class ChainRoles {
  chains = $state<DaemonChain[]>([]);
  loading = $state(true);
  loadError = $state<string | null>(null);
  /** Role → chain id — the daemon's truth after each write. */
  assignments = $state<Record<SenseiRole, string | null>>(byRole(null));
  /** Per role, so a slow write to one does not hide another's outcome. */
  saveStatus = $state<Record<SenseiRole, SaveStatus>>(byRole('idle'));
  saveError = $state<Record<SenseiRole, string | null>>(byRole(null));
  /** Per member (fallback_chain_models.id), so one slow move does not disable its neighbours. */
  memberBusy = $state<Record<string, 'up' | 'down' | 'remove' | null>>({});

  #api: () => Api;

  constructor(api: () => Api) {
    this.#api = api;
  }

  /** A failed read is said so (#288), never shown as every role unassigned. */
  async load(): Promise<void> {
    const r = await this.#api().tryListGatewayChains();
    if (r.ok) {
      this.chains = r.data.chains;
      this.assignments = roleAssignments(this.chains);
      this.loadError = null;
    } else {
      this.loadError = failureReason(r.error);
    }
    this.loading = false;
  }

  /** The active chains that can serve a role and are not another role's. */
  optionsFor(role: SenseiRole): DaemonChain[] {
    const caps = ROLE_META[role].capabilities;
    return this.chains.filter((c) => c.isActive && caps.includes(c.capability) && (c.role == null || c.role === role));
  }

  chainById(id: string | null): DaemonChain | undefined {
    return id ? this.chains.find((c) => c.id === id) : undefined;
  }

  async pick(role: SenseiRole, next: string | null): Promise<void> {
    const prev = this.assignments[role];
    if ((next ?? null) === (prev ?? null)) return;
    this.#saving(role);
    const refused = await assignChainRoles(this.#api(), [{ role, prev, next }]);
    if (refused.length) {
      this.#failed(role, refused.join('; '));
      // Show what actually stuck.
      await this.load();
      return;
    }
    this.assignments[role] = next;
    this.#saved(role);
  }

  moveMember(role: SenseiRole, memberId: string, direction: -1 | 1): Promise<void> {
    return this.#member(role, memberId, direction === -1 ? 'up' : 'down', (chainId) =>
      this.#api().moveGatewayChainModel(chainId, memberId, direction),
    );
  }

  removeMember(role: SenseiRole, memberId: string): Promise<void> {
    return this.#member(role, memberId, 'remove', (chainId) => this.#api().removeGatewayChainModel(chainId, memberId));
  }

  async #member(
    role: SenseiRole,
    memberId: string,
    busy: 'up' | 'down' | 'remove',
    write: (chainId: string) => Promise<{ ok: true } | { ok: false; error: { message: string } }>,
  ): Promise<void> {
    const chainId = this.assignments[role];
    if (!chainId) return;
    this.memberBusy = { ...this.memberBusy, [memberId]: busy };
    this.#saving(role);
    try {
      const r = await write(chainId);
      if (!r.ok) return this.#failed(role, r.error.message);
      await this.load();
      this.#saved(role);
    } finally {
      this.memberBusy = { ...this.memberBusy, [memberId]: null };
    }
  }

  #saving(role: SenseiRole): void {
    this.saveStatus[role] = 'saving';
    this.saveError[role] = null;
  }

  #failed(role: SenseiRole, message: string): void {
    this.saveStatus[role] = 'error';
    this.saveError[role] = message;
  }

  #saved(role: SenseiRole): void {
    this.saveStatus[role] = 'saved';
    setTimeout(() => {
      if (this.saveStatus[role] === 'saved') this.saveStatus[role] = 'idle';
    }, SAVED_MS);
  }
}
