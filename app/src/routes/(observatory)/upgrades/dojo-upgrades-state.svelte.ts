// Observatory · Upgrades — the Dōjō downstream lane.
//
// Co-locates the presentation logic for the `/api/upgrades` inbox so the
// `+page.svelte` template stays pure: type -> pill, state -> chip, scope /
// attribution -> copy, pinned-first ordering, unread count, and the
// Apply / Mute / Pin action controller. The daemon owns the inbox and its
// ordering (`crates/senseid/src/collective/inbox.rs::order_and_filter`); this
// module mirrors that ordering client-side so the list stays stable between
// invalidations and is unit-tested without a daemon.
//
// Wire-API-wins: `type`, `state` and `attribution.mode` arrive as free strings,
// so an unrecognised value degrades to a neutral chip rather than being dropped
// -- mirroring the dojo-connections state module's discipline.

import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { DojoUpgrade, DojoUpgradeAttribution, DojoUpgradeScope } from '$lib/types.js';

/** Named-token utility classes for a chip, plus its printed label. Mirrors the
 *  dojo-connections `ChipClasses` shape so the two Dōjō surfaces read
 *  consistently. */
export interface ChipClasses {
  bg: string;
  text: string;
  label: string;
}

/** A type pill -- kanji glyph + label + named-token classes. */
export interface TypePill {
  kanji: string;
  label: string;
  bg: string;
  text: string;
}

/** Pure: artifact type -> kanji glyph + label + chip classes. skill/agent read
 *  accent (concrete, installable operators), principle reads success, pattern /
 *  prompt read info, guard reads warning; an unrecognised type degrades to muted
 *  ink so a new wire kind is still shown, never dropped. Kanji follow the
 *  dojo-protocol payload glyphs and the local buckets vocabulary (技 skill,
 *  作 agent). The lookup IS the rendering rule, so it lives with the domain
 *  rather than in the component. */
export function typePill(type: string): TypePill {
  switch (type) {
    case 'skill':     return { kanji: '技', label: 'skill',     bg: 'bg-accent-soft',  text: 'text-accent'  };
    case 'agent':     return { kanji: '作', label: 'agent',     bg: 'bg-accent-soft',  text: 'text-accent'  };
    case 'principle': return { kanji: '理', label: 'principle', bg: 'bg-success-soft', text: 'text-success' };
    case 'pattern':   return { kanji: '紋', label: 'pattern',   bg: 'bg-info-soft',    text: 'text-info'    };
    case 'prompt':    return { kanji: '問', label: 'prompt',    bg: 'bg-info-soft',    text: 'text-info'    };
    case 'guard':     return { kanji: '守', label: 'guard',     bg: 'bg-warning-soft', text: 'text-warning' };
    default:          return { kanji: '贈', label: type || 'upgrade', bg: 'bg-paper-mute', text: 'text-ink-mute' };
  }
}

/** Semantic tone an inbox state maps to. `ink` = an unexpected state we still
 *  surface (never a status colour). */
export type StateTone = 'info' | 'success' | 'accent' | 'ink';

/** Pure: inbox state -> semantic tone. `pending` awaits a decision (info),
 *  `applied` landed (success), `pinned` floats / outranks (accent); `muted` and
 *  any unknown state degrade to muted ink. */
export function stateTone(state: string): StateTone {
  switch (state) {
    case 'pending': return 'info';
    case 'applied': return 'success';
    case 'pinned':  return 'accent';
    default:        return 'ink'; // muted + unknown
  }
}

/** Pure: named-token chip classes + label for an inbox state. The label is the
 *  raw wire state (honest -- a chip never claims a state the daemon didn't). */
export function stateChip(state: string): ChipClasses {
  switch (stateTone(state)) {
    case 'info':    return { bg: 'bg-info-soft',    text: 'text-info',     label: state };
    case 'success': return { bg: 'bg-success-soft', text: 'text-success',  label: state };
    case 'accent':  return { bg: 'bg-accent-soft',  text: 'text-accent',   label: state };
    default:        return { bg: 'bg-paper-mute',   text: 'text-ink-mute', label: state };
  }
}

/** Pure: the most specific scope tag as a short label, e.g. "project · lumen",
 *  "stack · rust", "team · mobile", "company · acme", else "all projects".
 *  Mirrors the daemon's scope precedence (project -> stack -> ...). */
export function scopeSummary(scope: DojoUpgradeScope | null | undefined): string {
  if (!scope) return 'all projects';
  if (scope.project?.trim()) return `project · ${scope.project.trim()}`;
  if (scope.stack?.trim())   return `stack · ${scope.stack.trim()}`;
  if (scope.team?.trim())    return `team · ${scope.team.trim()}`;
  if (scope.company?.trim()) return `company · ${scope.company.trim()}`;
  return 'all projects';
}

/** Pure: attribution -> a short "where it came from" line. Named credits the
 *  author (and org when present); anonymous / dereferenced say so; anything else
 *  echoes the raw mode so provenance is never blank. */
export function attributionSummary(attr: DojoUpgradeAttribution | null | undefined): string {
  if (!attr) return 'unattributed';
  switch (attr.mode) {
    case 'named': {
      const author = attr.author?.trim();
      const org = attr.org?.trim();
      if (author && org) return `by ${author} · ${org}`;
      if (author) return `by ${author}`;
      if (org) return `from ${org}`;
      return 'named';
    }
    case 'anonymous':    return 'anonymous';
    case 'dereferenced': return 'source withheld';
    default:             return attr.mode || 'unattributed';
  }
}

/** Pure: an inbox item's received date as a short YYYY-MM-DD label, or '' when
 *  absent / unparseable. Takes the date portion of the RFC-3339 string directly
 *  (no `Date` construction) so the display-only label never shifts by timezone. */
export function receivedLabel(received_at: string | null | undefined): string {
  if (!received_at) return '';
  const m = /^(\d{4}-\d{2}-\d{2})/.exec(received_at.trim());
  return m ? m[1] : '';
}

/** Pure: pinned-first ordering, then most-recent first -- the same contract the
 *  daemon applies in `order_and_filter`, mirrored so the list is stable across
 *  invalidations. Does not mutate the input. */
export function orderUpgrades(items: DojoUpgrade[]): DojoUpgrade[] {
  const pinned = (i: DojoUpgrade): number => (i.state === 'pinned' ? 1 : 0);
  return [...items].sort(
    (a, b) => pinned(b) - pinned(a) || (b.received_at ?? '').localeCompare(a.received_at ?? ''),
  );
}

/** Pure: unread = items still awaiting a decision (`pending`). Mirrors the
 *  daemon's `unread_count`. */
export function unreadCount(items: DojoUpgrade[]): number {
  return items.filter((i) => i.state === 'pending').length;
}

/**
 * The Apply / Mute / Pin controller. Tracks which item is mid-flight and the
 * last error so the `+page.svelte` stays a pure template. The api client and
 * the reload callback are injected, so the controller is unit-tested with a
 * hand-rolled mock and never reaches for a global. Each action resolves `true`
 * on success -- after the injected `reload` re-loads the list -- and `false` on
 * a wire failure, in which case `error` carries the reason and the row is left
 * in place for a retry.
 */
export class UpgradeActions {
  /** The id currently mid-flight, or null. Drives per-row button disabling. */
  busyId = $state<string | null>(null);
  /** The last action error, or null. Cleared at the start of each action. */
  error = $state<string | null>(null);

  #api: SenseiApi;
  #reload: () => Promise<void> | void;

  constructor(api: SenseiApi, reload: () => Promise<void> | void) {
    this.#api = api;
    this.#reload = reload;
  }

  /** True while `id` is the item mid-flight. */
  isBusy(id: string): boolean {
    return this.busyId === id;
  }

  async #run(id: string, call: () => Promise<ApiResult<unknown>>): Promise<boolean> {
    if (this.busyId) return false; // one decision at a time
    this.busyId = id;
    this.error = null;
    const res = await call();
    this.busyId = null;
    if (res.ok) {
      await this.#reload();
      return true;
    }
    this.error = res.error.message;
    return false;
  }

  apply(id: string): Promise<boolean> {
    return this.#run(id, () => this.#api.applyUpgrade(id));
  }

  mute(id: string): Promise<boolean> {
    return this.#run(id, () => this.#api.muteUpgrade(id));
  }

  pin(id: string): Promise<boolean> {
    return this.#run(id, () => this.#api.pinUpgrade(id));
  }
}
