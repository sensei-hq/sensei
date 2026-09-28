// Observatory · Collective (群) — the "what leaves the machine and toward whom"
// controls. The daemon owns the stored row + its defaults
// (`GET/PUT /api/preferences/collective`); this module holds the full wire
// object and does read-modify-write: mutate one field, PUT the WHOLE object,
// then adopt the returned saved object as the new truth. It also maps the flat
// `destination` enum to the two-toggle (global / dojo) mental model and carries
// the cadence / category / attribution option metadata so the `+page.svelte`
// stays a pure template.
//
// Wire-API-wins: the daemon 400s on a bad enum / unknown category / non-boolean,
// so only valid values ever come back; the mapping helpers still take `string`
// and keep a default branch so an unexpected value degrades rather than throws —
// mirroring the dojo-connections / dojo-upgrades state modules' discipline.

import { isoDay } from '$lib/dates.js';
import type { SenseiApi } from '$lib/api.js';
import type {
  CollectiveAttribution,
  CollectiveCadence,
  CollectiveCategoryKey,
  CollectiveDestination,
  CollectivePreferences,
} from '$lib/types.js';

// ── Destination <-> two-toggle mapping ───────────────────────────────────────
// The contract models the destination as a single 4-state enum, but the UI (and
// the mockup's mental model) is two coexisting toggles: the global commons and
// the company Dōjō. `both` = both on, `none` = both off. These two pure helpers
// are the whole of that translation, so it is testable without the daemon.

/** Pure: two toggle states -> the destination enum the daemon stores. */
export function destinationFromToggles(global: boolean, dojo: boolean): CollectiveDestination {
  if (global && dojo) return 'both';
  if (global) return 'global';
  if (dojo) return 'dojo';
  return 'none';
}

/** Pure: the destination enum -> the two toggle states. `none` and any
 *  unexpected value read as both-off (never a broken toggle). */
export function togglesFromDestination(dest: string): { global: boolean; dojo: boolean } {
  switch (dest) {
    case 'both':
      return { global: true, dojo: true };
    case 'global':
      return { global: true, dojo: false };
    case 'dojo':
      return { global: false, dojo: true };
    default:
      return { global: false, dojo: false }; // 'none' + unknown
  }
}

// ── Pure read-modify-write builders ──────────────────────────────────────────
// Each returns a NEW full preferences object with one field changed and the
// input left untouched, so the whole-object PUT always carries every field and a
// failed save never half-mutates local state.

/** Pure: whole object with a new destination. */
export function withDestination(
  p: CollectivePreferences,
  destination: CollectiveDestination,
): CollectivePreferences {
  return { ...p, destination };
}

/** Pure: flip the global-commons toggle, preserving the dojo toggle. */
export function setGlobal(p: CollectivePreferences, on: boolean): CollectivePreferences {
  const { dojo } = togglesFromDestination(p.destination);
  return withDestination(p, destinationFromToggles(on, dojo));
}

/** Pure: flip the company-Dōjō toggle, preserving the global toggle. */
export function setDojo(p: CollectivePreferences, on: boolean): CollectivePreferences {
  const { global } = togglesFromDestination(p.destination);
  return withDestination(p, destinationFromToggles(global, on));
}

/** Pure: whole object with a new cadence. */
export function withCadence(
  p: CollectivePreferences,
  cadence: CollectiveCadence,
): CollectivePreferences {
  return { ...p, cadence };
}

/** Pure: whole object with a new default attribution. */
export function withAttribution(
  p: CollectivePreferences,
  attribution_default: CollectiveAttribution,
): CollectivePreferences {
  return { ...p, attribution_default };
}

/** Pure: whole object with exactly ONE category key flipped; every other key
 *  (and the rest of the object) is preserved. */
export function withCategory(
  p: CollectivePreferences,
  key: CollectiveCategoryKey,
  on: boolean,
): CollectivePreferences {
  return { ...p, categories: { ...p.categories, [key]: on } };
}

// ── Option metadata (rendering rules — live with the domain) ──────────────────

/** A destination toggle — the two coexisting share targets. */
export interface DestinationToggleMeta {
  key: 'global' | 'dojo';
  kanji: string;
  label: string;
  blurb: string;
}

/** The two destinations, in the order the mockup shows them. `結` mirrors the
 *  Dōjō rail glyph; `群` is the collective. */
export const DESTINATION_TOGGLES: readonly DestinationToggleMeta[] = [
  {
    key: 'global',
    kanji: '群',
    label: 'Global commons',
    blurb: 'the public collective — anonymized insight that helps every sensei',
  },
  {
    key: 'dojo',
    kanji: '結',
    label: 'Company Dōjō',
    blurb: 'the teams you have paired with under connections',
  },
];

/** Cadence chips, single-select. Lowercase — voice charter. */
export const CADENCE_OPTIONS: readonly { value: CollectiveCadence; label: string }[] = [
  { value: 'manual', label: 'manual' },
  { value: 'daily', label: 'daily' },
  { value: 'weekly', label: 'weekly' },
];

/** One share category — kanji glyph + label + one-line blurb. */
export interface CategoryMeta {
  key: CollectiveCategoryKey;
  kanji: string;
  label: string;
  blurb: string;
}

/** The seven categories in the wire's stable key order. */
export const CATEGORY_META: readonly CategoryMeta[] = [
  { key: 'memory', kanji: '覚', label: 'Memories', blurb: 'distilled lessons from your sessions' },
  { key: 'pattern', kanji: '紋', label: 'Patterns', blurb: 'recurring shapes across your projects' },
  { key: 'rule', kanji: '則', label: 'Rules', blurb: 'conventions you hold your work to' },
  { key: 'prompt', kanji: '問', label: 'Prompts', blurb: 'prompt shapes that reliably worked' },
  { key: 'guard', kanji: '守', label: 'Guards', blurb: 'checks that catch mistakes early' },
  { key: 'skill', kanji: '技', label: 'Skills', blurb: 'skills you have authored' },
  { key: 'agent', kanji: '作', label: 'Agents', blurb: 'agents you have authored' },
];

/** One attribution option — value + label + a one-line description. */
export interface AttributionMeta {
  value: CollectiveAttribution;
  label: string;
  blurb: string;
}

/** Attribution defaults, safest last (which is the wire default). */
export const ATTRIBUTION_OPTIONS: readonly AttributionMeta[] = [
  { value: 'named', label: 'Named', blurb: 'keep authorship — your name travels with the insight' },
  { value: 'anonymous', label: 'Anonymous', blurb: 'drop your identity, keep the content' },
  {
    value: 'dereferenced',
    label: 'Dereferenced',
    blurb: 'strip the source reference — the safest, and the default',
  },
];

// ── Small display helpers ────────────────────────────────────────────────────

/** Pure: the date portion of an RFC-3339 string, or '' when absent /
 *  unparseable. Delegates to the shared `isoDay` — this was one of three copies
 *  of the same three lines — and keeps the empty-string contract its callers
 *  render directly. */
export function collectiveDateLabel(iso: string | null | undefined): string {
  return isoDay(iso) ?? '';
}

/** Pure: the "last changed …" line, or a defaults hint when never saved. */
export function updatedLabel(updated_at: string | null | undefined): string {
  const d = collectiveDateLabel(updated_at);
  return d ? `last changed ${d}` : 'using defaults · never changed';
}

/**
 * The Collective sharing controller. Holds the full wire object as the single
 * source of truth and does read-modify-write on every toggle: build the next
 * whole object (pure), PUT it, and adopt the server-returned saved object. The
 * api client is injected as the seam, so the controller is unit-tested with a
 * hand-rolled mock and never reaches for a global.
 *
 * A save resolves `true` on success (the saved object is adopted) and `false` on
 * a wire failure, in which case `error` carries the daemon's message and
 * `current` is left untouched — so the toggle visually reverts and nothing is
 * half-applied (wire-API-wins).
 */
export class CollectiveSharing {
  /** The full wire object — the single source of truth for the whole screen. */
  current = $state<CollectivePreferences>({
    destination: 'none',
    cadence: 'manual',
    categories: { memory: true, pattern: true, rule: true, prompt: true, guard: true, skill: true, agent: true },
    attribution_default: 'dereferenced',
    updated_at: null,
  });
  /** True while a PUT is in flight — drives control disabling / feedback. */
  saving = $state(false);
  /** The last save error, or null. Cleared at the start of each save. */
  error = $state<string | null>(null);

  #api: SenseiApi;

  constructor(initial: CollectivePreferences, api: SenseiApi) {
    this.current = initial;
    this.#api = api;
  }

  get destination(): CollectiveDestination {
    return this.current.destination;
  }
  get cadence(): CollectiveCadence {
    return this.current.cadence;
  }
  get attribution(): CollectiveAttribution {
    return this.current.attribution_default;
  }
  /** True when either destination is enabled — i.e. something can leave. */
  get sharing(): boolean {
    return this.current.destination !== 'none';
  }
  get globalOn(): boolean {
    return togglesFromDestination(this.current.destination).global;
  }
  get dojoOn(): boolean {
    return togglesFromDestination(this.current.destination).dojo;
  }
  get updatedLabel(): string {
    return updatedLabel(this.current.updated_at);
  }
  get neverSaved(): boolean {
    return this.current.updated_at === null;
  }

  /** Whether a category is currently on. */
  isCategoryOn(key: CollectiveCategoryKey): boolean {
    return this.current.categories[key];
  }

  setGlobal(on: boolean): Promise<boolean> {
    return this.#save(setGlobal(this.current, on));
  }
  setDojo(on: boolean): Promise<boolean> {
    return this.#save(setDojo(this.current, on));
  }
  setCadence(cadence: CollectiveCadence): Promise<boolean> {
    return this.#save(withCadence(this.current, cadence));
  }
  setAttribution(attribution: CollectiveAttribution): Promise<boolean> {
    return this.#save(withAttribution(this.current, attribution));
  }
  setCategory(key: CollectiveCategoryKey, on: boolean): Promise<boolean> {
    return this.#save(withCategory(this.current, key, on));
  }
  toggleCategory(key: CollectiveCategoryKey): Promise<boolean> {
    return this.setCategory(key, !this.current.categories[key]);
  }

  async #save(next: CollectivePreferences): Promise<boolean> {
    if (this.saving) return false; // one whole-object write at a time
    this.saving = true;
    this.error = null;
    const res = await this.#api.putCollectivePreferences(next);
    this.saving = false;
    if (res.ok) {
      this.current = res.data; // adopt the server-returned saved object as truth
      return true;
    }
    this.error = res.error.message; // current untouched → the control reverts
    return false;
  }
}
