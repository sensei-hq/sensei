// Settings · Dōjō connection — the screen's state.
//
// One state decides; the components render (frontend guidelines §2). This class
// owns which identity is selected, the repository filter, and every line the
// screen says. Facts come in through one getter — identities from PersonaList,
// memberships and sync rows from the page's load — so the screen never holds a
// second copy of either; actions go back out to PersonaList.
//
// Every action for an identity lives in the header (user, 2026-11-03): the
// cards show status only, so there is one place to act.

import type { DojoMembership, SyncStateRow } from '$lib/types.js';
import { IDENTITY_TONE_CLASS, type Identity, type IdentitiesPhase } from '$lib/personas.svelte.js';
import {
  REPO_FILTERS,
  dojoGroups,
  identitySummary,
  isConnected,
  type DojoGroup,
  type RepoFilter,
} from './dojo-connection.js';
import { agreementLine, entityLabel, syncToneClass } from './dojo-status-state.svelte.js';

export interface ConnectionSource {
  identities: Identity[];
  phase: IdentitiesPhase;
  /** PersonaList.error: a failed read (phase 'error') or a failed action. */
  identitiesError: string | null;
  /** `null` with `membershipsError` set when the read failed — never `[]`. */
  memberships: DojoMembership[] | null;
  membershipsError: string | null;
  sync: SyncStateRow[] | null;
  syncError: string | null;
}

export interface ConnectionActions {
  act: (id: Identity) => unknown;
  makeDefault: (id: Identity) => unknown;
  signOut: (id: Identity) => unknown;
}

/** Notice props the template passes through (packages/ui/Notice.svelte). */
export interface ScreenNoticeProps {
  tone: 'fault' | 'refusal' | 'action' | 'normal';
  title: string;
  detail?: string;
  remedy?: string;
  code?: string;
}

export interface IdentityCard {
  label: string;
  name: string;
  isDefault: boolean;
  standing: string;
  toneClass: string;
  selected: boolean;
  summary: string | null;
}

export interface IdentityHeader {
  name: string;
  login: string;
  standing: string;
  toneClass: string;
  primary: { label: string; busy: boolean } | null;
  canMakeDefault: boolean;
  canSignOut: boolean;
  busy: boolean;
}

const DAEMON_REMEDY = 'Check that the sensei daemon is running, then retry.';

/** An identity's own sync entities — about the identity, not a repository. */
const IDENTITY_ENTITIES = new Set(['forge_mirror', 'dojo_sync_plan']);

export interface ActivityLine {
  key: string;
  label: string;
  line: string;
  toneClass: string;
}

export class DojoConnectionScreen {
  #source: () => ConnectionSource;
  #actions: ConnectionActions;
  /** The reader's pick, by label; the default identity until there is one. */
  chosen = $state<string | null>(null);
  filter = $state<RepoFilter>('all');

  constructor(source: () => ConnectionSource, actions: ConnectionActions) {
    this.#source = source;
    this.#actions = actions;
  }

  get #src(): ConnectionSource {
    return this.#source();
  }

  get selected(): Identity | null {
    const ids = this.#src.identities;
    return ids.find((i) => i.label === this.chosen) ?? ids.find((i) => i.isDefault) ?? ids[0] ?? null;
  }

  select(label: string): void {
    this.chosen = label;
  }

  setFilter(filter: RepoFilter): void {
    this.filter = filter;
  }

  get cards(): IdentityCard[] {
    const sel = this.selected?.label;
    const { memberships, sync } = this.#src;
    return this.#src.identities.map((i) => ({
      label: i.label,
      name: i.name,
      isDefault: i.isDefault,
      standing: i.standing,
      toneClass: IDENTITY_TONE_CLASS[i.tone],
      selected: i.label === sel,
      summary: identitySummary(i, memberships ?? [], sync ?? []),
    }));
  }

  get header(): IdentityHeader | null {
    const i = this.selected;
    if (!i) return null;
    const connected = isConnected(i);
    const label = i.action?.kind === 'connect' ? (i.busy ? 'opening…' : 'Connect with GitHub') : i.buttonLabel;
    return {
      name: i.name,
      login: i.githubLogin ? `@${i.githubLogin} on GitHub` : 'Inferred from commit authorship',
      standing: i.standing,
      toneClass: IDENTITY_TONE_CLASS[i.tone],
      primary: i.action && label ? { label, busy: i.busy } : null,
      canMakeDefault: connected && !i.isDefault,
      canSignOut: i.sessionSlot !== null && i.status !== 'neverConnected',
      busy: i.busy,
    };
  }

  runPrimary(): void {
    const i = this.selected;
    if (i?.action) void this.#actions.act(i);
  }

  runMakeDefault(): void {
    const i = this.selected;
    if (i) void this.#actions.makeDefault(i);
  }

  runSignOut(): void {
    const i = this.selected;
    if (i) void this.#actions.signOut(i);
  }

  /** What the detail pane shows under its header. */
  get body(): 'connect' | 'attention' | 'dojos' | null {
    const i = this.selected;
    if (!i) return null;
    if (isConnected(i)) return 'dojos';
    return i.status === 'neverConnected' ? 'connect' : 'attention';
  }

  get connectSteps(): string[] {
    const i = this.selected;
    const who = i?.githubLogin ?? i?.name ?? '';
    return [
      `Sign in as @${who} in the browser that opens. A different GitHub account is refused.`,
      'Every dōjō this account belongs to joins here: its personal dōjō and any organizations.',
      "Your personal repositories sync by default and you can turn any off. An organization's are set by its admin in the dōjō.",
    ];
  }

  /** An identity that needs a hand: what happened and what to do — the button
   *  that does it is the header's. */
  get attention(): ScreenNoticeProps | null {
    const i = this.selected;
    if (!i || this.body !== 'attention') return null;
    if (i.status === 'needsKeychain') {
      return { tone: 'action', title: `macOS Keychain is holding ${i.name}'s credential`, remedy: 'Approve Keychain access above. When macOS asks, choose Always Allow.' };
    }
    return { tone: 'action', title: `${i.name}'s credential expired`, remedy: 'Sign in again above. Nothing synced so far is lost; syncing resumes once you are back.' };
  }

  get dojos(): DojoGroup[] {
    const i = this.selected;
    if (!i || !isConnected(i)) return [];
    return dojoGroups(this.#src.memberships ?? [], this.#src.sync ?? [], i.sessionSlot, this.filter);
  }

  get #all(): DojoGroup[] {
    const i = this.selected;
    if (!i || !isConnected(i)) return [];
    return dojoGroups(this.#src.memberships ?? [], this.#src.sync ?? [], i.sessionSlot, 'all');
  }

  get filters(): { key: RepoFilter; label: string; count: number; on: boolean }[] {
    const repos = this.#all.flatMap((g) => g.repos);
    return REPO_FILTERS.map((f) => ({
      ...f,
      count: f.key === 'all' ? repos.length : repos.filter((r) => r.category === f.key).length,
      on: this.filter === f.key,
    }));
  }

  get summary(): string {
    const groups = this.#all.filter((g) => g.id !== 'unfiled');
    const repos = this.#all.flatMap((g) => g.repos);
    const syncing = repos.filter((r) => r.category === 'syncing').length;
    const n = (k: number, one: string, many: string) => `${k} ${k === 1 ? one : many}`;
    return `${n(groups.length, 'dōjō', 'dōjōs')} · ${n(repos.length, 'repository', 'repositories')} · ${syncing} syncing`;
  }

  /** Connected, read fine, and no dōjō lists this identity — said as a normal
   *  wait, not left blank. `null` when the read failed: that is a different answer. */
  get noDojos(): ScreenNoticeProps | null {
    const i = this.selected;
    if (!i || this.body !== 'dojos' || this.#src.memberships === null) return null;
    if (this.#src.memberships.some((m) => m.persona_slot === i.sessionSlot)) return null;
    return {
      tone: 'normal',
      title: `No dōjō lists @${i.githubLogin ?? i.name} yet`,
      detail: 'Dōjōs join here on the next sync after an organization adds this account.',
    };
  }

  /** Reads that failed, each said on its own. */
  get loadNotices(): ScreenNoticeProps[] {
    const out: ScreenNoticeProps[] = [];
    const s = this.#src;
    if (s.membershipsError !== null) {
      out.push({ tone: 'fault', title: "Couldn't load your dōjōs", detail: s.membershipsError || undefined, remedy: DAEMON_REMEDY, code: 'memberships' });
    }
    if (s.syncError !== null) {
      out.push({ tone: 'fault', title: "Couldn't load what syncs", detail: s.syncError || undefined, remedy: DAEMON_REMEDY, code: 'sync_state' });
    }
    return out;
  }

  /** The selected identity's own rows (its sync plans, its forge mirror),
   *  keyed by entity · key · direction — two dōjōs' plans share an entity. */
  get #ownRows(): SyncStateRow[] {
    const slot = this.selected?.sessionSlot;
    if (!slot) return [];
    return (this.#src.sync ?? []).filter((r) => r.persona === slot && IDENTITY_ENTITIES.has(r.entity));
  }

  get activity(): ActivityLine[] {
    const rank = (r: SyncStateRow) => (r.state === 'error' ? 0 : 1);
    return [...this.#ownRows]
      .sort((a, b) => rank(a) - rank(b))
      .map((r) => ({
        key: `${r.entity}:${r.entity_key}:${r.direction}`,
        label: `${entityLabel(r.entity)} · ${r.direction}`,
        line: agreementLine(r),
        toneClass: syncToneClass(r.state),
      }));
  }

  /** The identity's own failures, as fault Notices in their own words. */
  get identityIssues(): (ScreenNoticeProps & { key: string })[] {
    return this.#ownRows
      .filter((r) => r.state === 'error')
      .map((r) => ({
        key: `${r.entity}:${r.entity_key}:${r.direction}`,
        tone: 'fault' as const,
        title: `${entityLabel(r.entity)} is failing`,
        detail: r.last_error ?? undefined,
        code: r.entity,
      }));
  }

  /** A failed registry read replaces the list; a failed action sits beside it. */
  get identitiesNotice(): ScreenNoticeProps | null {
    const { phase, identitiesError } = this.#src;
    if (!identitiesError) return null;
    if (phase === 'error') return { tone: 'fault', title: "Couldn't read your identities", detail: identitiesError, code: 'personas' };
    return { tone: 'fault', title: "That didn't go through", detail: identitiesError, code: 'identity_action' };
  }
}
