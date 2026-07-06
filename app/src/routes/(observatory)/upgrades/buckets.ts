/** One installable upgrade — an analyzer recommendation whose action_type
 *  points at a concrete artifact (skill / agent / rule / lint). */
export interface Upgrade {
  id: string;
  projectId: string;
  projectName: string;
  title: string;
  why: string;
  urgency: 'high' | 'medium' | 'low';
  status: 'pending' | 'accepted' | 'dismissed' | 'superseded';
  actionType: string;
}

/** The upgrade categories the mockup surfaces, keyed to the analyzer's
 *  action_type vocabulary. `other` collects any action_type we don't yet
 *  have a bucket for — surfaces the row instead of silently dropping it. */
export type UpgradeKind = 'skill' | 'agent' | 'rule' | 'lint' | 'other';

export interface UpgradeBuckets {
  skill: Upgrade[];
  agent: Upgrade[];
  rule:  Upgrade[];
  lint:  Upgrade[];
  other: Upgrade[];
}

/** The single-glyph icon and human label per bucket — kept next to the
 *  bucketer so a new action_type only touches one file. */
export const KIND_META: Record<UpgradeKind, { kanji: string; title: string; sub: string }> = {
  skill: { kanji: '技', title: 'Skills',   sub: 'reusable how-to files sensei can inject when relevant' },
  agent: { kanji: '作', title: 'Agents',   sub: 'multi-step operators that own a task' },
  rule:  { kanji: '則', title: 'Rules',    sub: 'inline guardrails the analyzer wants to promote' },
  lint:  { kanji: '禁', title: 'Lints',    sub: 'anti-patterns worth flagging automatically' },
  other: { kanji: '他', title: 'Other',    sub: 'recommendations that don\'t map to a known category' },
};

/** Pure: bucket an action_type into its upgrade kind. Kept exported so
 *  tests + the loader stay in lockstep. */
export function kindFor(actionType: string): UpgradeKind {
  switch (actionType) {
    case 'write_skill':    return 'skill';
    case 'create_agent':   return 'agent';
    case 'revise_rule':
    case 'enrich_memory':  return 'rule';
    case 'audit_stale':    return 'lint';
    default:               return 'other';
  }
}

/** Pure: bucket upgrades. Each bucket is sorted urgency-desc (high →
 *  medium → low) so the reader hits the loudest signal first, ties
 *  broken by project name for determinism. */
export function bucketUpgrades(upgrades: Upgrade[]): UpgradeBuckets {
  const buckets: UpgradeBuckets = { skill: [], agent: [], rule: [], lint: [], other: [] };
  for (const u of upgrades) buckets[kindFor(u.actionType)].push(u);
  const rank = { high: 0, medium: 1, low: 2 } as const;
  for (const key of Object.keys(buckets) as UpgradeKind[]) {
    buckets[key].sort((a, b) =>
      rank[a.urgency] - rank[b.urgency] || a.projectName.localeCompare(b.projectName),
    );
  }
  return buckets;
}
