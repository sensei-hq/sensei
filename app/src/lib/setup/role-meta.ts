import type { SenseiRole, DaemonChain } from './contracts.js';

/** Presentation + picker-filter metadata for each sensei role. Kept
 *  co-located with the contract so a new role only touches one file.
 *
 *  `capabilities` gates which chains can serve a role — a chain's
 *  capability must be in this list for it to appear in the role's
 *  picker. */
export interface RoleMeta {
  kanji: string;
  label: string;
  hint: string;
  capabilities: string[];
}

export const ROLE_META: Record<SenseiRole, RoleMeta> = {
  inference: {
    kanji: '推',
    label: 'Inference',
    hint: 'insights, actions, and recommendations from sessions + memory',
    capabilities: ['reasoning'],
  },
  consolidation: {
    kanji: '洞',
    label: 'Consolidation',
    hint: 'merge memories, detect conflicts, propose scope updates',
    capabilities: ['reasoning', 'summarize'],
  },
  embedding: {
    kanji: '印',
    label: 'Embedding',
    hint: 'index sessions, memories, and code refs for retrieval',
    capabilities: ['embed'],
  },
  voice: {
    kanji: '話',
    label: 'Voice',
    hint: 'observatory speech (optional — leave unassigned to keep quiet)',
    capabilities: ['audio', 'chat'],
  },
};

/** Each role's chain as the daemon has it: the first chain that claims the
 *  role, or null. One fold for the Settings panel and the setup wizard. */
export function roleAssignments(chains: readonly DaemonChain[]): Record<SenseiRole, string | null> {
  const out: Record<SenseiRole, string | null> = { inference: null, consolidation: null, embedding: null, voice: null };
  for (const c of chains) {
    if (c.role != null && out[c.role] === null) out[c.role] = c.id;
  }
  return out;
}
