// Settings · Dōjō connection — how the identity-first screen reads (pure).
//
// The screen is for one thing: connecting an identity to its dōjōs, then seeing
// what syncs. Identities come first; a connected one lists its dōjōs (memberships
// bound to its session slot), each with the repositories it has filed. Every
// derivation is here so the components only render, and so a repository reads the
// same in the identity's summary, its dōjō's count and its own row.
//
// A repository is filed under the dōjō whose `org_slugs` cover its forge owner —
// the same rule the daemon's routing uses (`dojo/routing.rs::infer_binding`). One
// no membership covers is shown under "Not filed under a dōjō", never dropped.
//
// No dates: when a repository last agreed is the dōjō's business, not this
// screen's (user, 2026-11-03).

import type { DojoMembership, SyncStateRow } from '$lib/types.js';
import type { Identity } from '$lib/personas.svelte.js';
import type { NoticeTone } from '$lib/notice-tone.js';
import { kindPill } from '../../../(observatory)/dojo/connections/dojo-connections-state.svelte.js';
import { detailLine } from './dojo-status-state.svelte.js';

/** What travels, in which direction (docs/spec/dojo/daemon-sync.md §1). */
export const TRAVELS = [
  {
    direction: 'out',
    label: 'Sensei to dōjō',
    items: ['Repositories sensei has scanned', 'Metrics and diagrams', 'Insights and memories'],
    note: null,
  },
  {
    direction: 'both',
    label: 'Both ways',
    items: ['Workspaces and their repositories', 'Teams and roles', 'Integration connections'],
    note: 'Edit on either side. If both change, the dōjō wins and sensei tells you.',
  },
  {
    direction: 'in',
    label: 'Dōjō to sensei',
    items: [
      'Governance rules and standards',
      'Tickets and sprints, from GitHub Issues, Jira or Azure Boards',
      'Sprint targets and release gates',
      'Deployments and DORA',
    ],
    note: 'Trackers are read-only: nothing is written back to them.',
  },
] as const;

export type RepoCategory = 'syncing' | 'off' | 'failing';
export type RepoFilter = 'all' | RepoCategory;

export const REPO_FILTERS: { key: RepoFilter; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'syncing', label: 'Syncing' },
  { key: 'off', label: 'Not syncing' },
  { key: 'failing', label: 'Failing' },
];

export function matchesFilter(filter: RepoFilter, category: RepoCategory): boolean {
  return filter === 'all' || filter === category;
}

/** A Notice for a repository row (`size="inline"` in the template). */
export interface RepoNotice {
  tone: Exclude<NoticeTone, 'action'>;
  title: string;
  detail?: string;
  code: string;
}

export interface RepoView {
  key: string;
  name: string;
  state: string;
  status: string;
  category: RepoCategory;
  notice: RepoNotice | null;
}

export interface DojoGroup {
  id: string;
  name: string;
  kind: string;
  kanji: string;
  isOrg: boolean;
  repos: RepoView[];
  empty: boolean;
  /** `null` when the sync read failed: no number is drawn from a failed read. */
  count: string | null;
}

/** The forge owner of `host/owner/name`, lowercased; `null` with no owner. */
export function ownerOf(key: string): string | null {
  const parts = key.split('/');
  return parts.length >= 3 ? parts[1].toLowerCase() : null;
}

/** `owner/name` — the host is the same for every row and says nothing. */
export function repoName(key: string): string {
  const parts = key.split('/');
  return parts.length >= 3 ? parts.slice(1).join('/') : key;
}

const STATUS: Record<string, { status: string; category: RepoCategory }> = {
  synced: { status: 'Syncing', category: 'syncing' },
  partial: { status: 'Syncing', category: 'syncing' },
  pending: { status: 'Waiting for the next sync', category: 'syncing' },
  skipped: { status: 'Not syncing', category: 'off' },
  error: { status: 'Failing', category: 'failing' },
};

/** Pure: one repository verdict → its row. Skipped is a decision (a refusal, in
 *  the remedy's words); error is a fault (in the failure's own words). */
export function repoView(row: SyncStateRow): RepoView {
  const s = STATUS[row.state] ?? { status: row.state, category: 'off' as const };
  let notice: RepoNotice | null = null;
  if (row.state === 'skipped') {
    notice = { tone: 'refusal', title: detailLine(row) ?? 'Not syncing, by decision', code: row.last_error ?? 'skipped' };
  } else if (row.state === 'error') {
    notice = { tone: 'fault', title: "This repository's last sync failed", detail: row.last_error ?? undefined, code: 'sync_error' };
  } else if (row.state === 'partial' && row.last_error) {
    notice = { tone: 'normal', title: row.last_error, code: 'partial' };
  }
  return { key: row.entity_key, name: repoName(row.entity_key), state: row.state, status: s.status, category: s.category, notice };
}

const RANK: Record<RepoCategory, number> = { failing: 0, off: 1, syncing: 2 };
const byRankThenName = (a: RepoView, b: RepoView) => RANK[a.category] - RANK[b.category] || a.name.localeCompare(b.name);

/** The repository verdict rows one identity's syncs wrote — one per repository,
 *  the last one written winning, so a row key is never repeated (a duplicate key
 *  froze the window once, 2026-10-28). */
function repositoryRows(rows: SyncStateRow[], slot: string): SyncStateRow[] {
  const byKey = new Map<string, SyncStateRow>();
  for (const r of rows) {
    if (r.entity === 'repository' && r.persona === slot) byKey.set(r.entity_key, r);
  }
  return [...byKey.values()];
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** The tenant's last segment — `github/sensei-hq` reads as `sensei-hq`. */
function tenantName(m: DojoMembership): string {
  return m.tenant_key.split('/').filter(Boolean).at(-1) ?? m.tenant_key;
}

/**
 * Pure: the selected identity's dōjōs, each with its repositories under
 * `filter`. `slot` null (signed out) → nothing: its dōjōs are not this
 * machine's to show until it connects again.
 */
export function dojoGroups(memberships: DojoMembership[], rows: SyncStateRow[], slot: string | null, filter: RepoFilter): DojoGroup[] {
  if (!slot) return [];
  const mine = memberships.filter((m) => m.persona_slot === slot);
  const views = repositoryRows(rows, slot).map((r) => ({ view: repoView(r), owner: ownerOf(r.entity_key) }));
  const claimed = new Set<string>();

  const group = (id: string, name: string, kind: string, isOrg: boolean, all: RepoView[]): DojoGroup => {
    const shown = all.filter((r) => matchesFilter(filter, r.category)).sort(byRankThenName);
    const syncing = all.filter((r) => r.category === 'syncing').length;
    return { id, name, kind, kanji: kindPill(kind).kanji, isOrg, repos: shown, empty: shown.length === 0, count: `${syncing} of ${all.length} syncing` };
  };

  const out = mine.map((m) => {
    const covered = views.filter((v) => v.owner !== null && m.org_slugs.includes(v.owner));
    covered.forEach((v) => claimed.add(v.view.key));
    return group(m.id, tenantName(m), m.kind, m.kind !== 'personal', covered.map((v) => v.view));
  });
  const stray = views.filter((v) => !claimed.has(v.view.key)).map((v) => v.view);
  if (stray.length > 0) out.push(group('unfiled', 'Not filed under a dōjō', 'unfiled', false, stray));
  return out;
}

/** Signed in with a usable credential — the state in which its dōjōs show. An
 *  expired or Keychain-blocked identity shows what to do instead. */
export function isConnected(id: Identity): boolean {
  return id.sessionSlot !== null && (id.status === 'ready' || id.status === 'expiring');
}

/** The card's second line: how many dōjōs, how many repositories syncing. */
export function identitySummary(id: Identity, memberships: DojoMembership[], rows: SyncStateRow[]): string | null {
  if (!isConnected(id) || !id.sessionSlot) return null;
  const dojos = memberships.filter((m) => m.persona_slot === id.sessionSlot).length;
  const syncing = repositoryRows(rows, id.sessionSlot).filter((r) => repoView(r).category === 'syncing').length;
  return `${plural(dojos, 'dōjō', 'dōjōs')} · ${syncing} syncing`;
}
