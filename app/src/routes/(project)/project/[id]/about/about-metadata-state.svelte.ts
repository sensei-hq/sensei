// Project · About — editable project-metadata form (P1-S14).
//
// The daemon owns the write: PUT /api/projects/{id} does a partial (COALESCE)
// update of the editable identity subset. This module owns the *view* state so
// the `+page.svelte` stays a pure template:
//   - the reactive form fields (name/client/goal/description/maturity),
//   - `hydrate()` — seed the fields from the wire project (the /api/projects
//     list payload, which serializes goal as `vision`),
//   - `toPatch()` — the lossless patch body (empty strings dropped so an unset
//     field never clobbers a stored value), and
//   - `save()` — the save controller + its idle/saving/saved/error lifecycle,
//     mirroring the settings-general form's `trySetConfig` flow.
//
// The api client is passed into `save` as the seam, so the state is unit-
// testable with a hand-rolled mock and never reaches for a global.

import type { SenseiApi } from '$lib/api.js';

/** The About form's save lifecycle — mirrors the settings-general form. */
export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

/** A selectable maturity — the wire enum value + its display label. */
export interface MaturityOption {
  value: string;
  label: string;
}

/**
 * The `sensei.project_maturity` enum, in ladder order (discovery → active →
 * maintenance → archived). Single source of the select's options; the daemon
 * re-validates against its own `PROJECT_MATURITIES` on the write so an unknown
 * value is a 400, never a 500.
 */
export const MATURITY_OPTIONS: readonly MaturityOption[] = [
  { value: 'discovery', label: 'Discovery' },
  { value: 'active', label: 'Active' },
  { value: 'maintenance', label: 'Maintenance' },
  { value: 'archived', label: 'Archived' },
];

const DEFAULT_MATURITY = 'discovery';

/** The minimal project shape the form reads. `goal` is the canonical key; the
 *  `/api/projects` list endpoint serializes it as `vision`, so both are
 *  accepted. All identity fields are optional/nullable so any project payload
 *  hydrates without a crash. */
export interface EditableProject {
  id: string;
  name?: string | null;
  client?: string | null;
  goal?: string | null;
  /** Alias the list endpoint uses for `goal`. */
  vision?: string | null;
  description?: string | null;
  maturity?: string | null;
}

/** Pure: the save-status → copy mapping, so the template holds no strings. */
export function saveStatusLabel(status: SaveStatus, error?: string | null): string {
  switch (status) {
    case 'saving':
      return 'saving…';
    case 'saved':
      return 'saved';
    case 'error':
      return error && error.length > 0 ? error : 'save failed';
    default:
      return 'auto-saves as you edit';
  }
}

/**
 * Reactive model + save controller for the About metadata form. Fields are
 * `$state` so `bind:` in the template drives them; `save` owns the network call
 * and the status transitions around it.
 */
export class ProjectMetadataForm {
  name = $state('');
  client = $state('');
  goal = $state('');
  description = $state('');
  maturity = $state(DEFAULT_MATURITY);

  saveStatus = $state<SaveStatus>('idle');
  saveError = $state<string | null>(null);

  /** Seed the fields from a freshly loaded project. Null/undefined wire values
   *  coerce to empty strings; `goal` falls back to the list endpoint's
   *  `vision` alias; `maturity` defaults to `discovery`. Also clears any prior
   *  save state so a re-hydrate (navigation) starts clean. */
  hydrate(p: EditableProject): void {
    this.name = p.name ?? '';
    this.client = p.client ?? '';
    this.goal = p.goal ?? p.vision ?? '';
    this.description = p.description ?? '';
    this.maturity = p.maturity ?? DEFAULT_MATURITY;
    this.saveStatus = 'idle';
    this.saveError = null;
  }

  /** The lossless patch body: every non-empty field. Empty strings are dropped
   *  so an untouched field never overwrites a stored value (the daemon
   *  COALESCE-merges only the keys present). */
  toPatch(): Record<string, string> {
    const patch: Record<string, string> = {};
    const fields: Record<string, string> = {
      name: this.name,
      client: this.client,
      goal: this.goal,
      description: this.description,
      maturity: this.maturity,
    };
    for (const [k, v] of Object.entries(fields)) {
      if (v && v.length > 0) patch[k] = v;
    }
    return patch;
  }

  /**
   * Persist the current form via the daemon. Transitions
   * saving → saved (auto-reverts to idle in the template's timer) or → error.
   * Returns `true` on success, `false` on failure or when there's no project
   * id. The api client is the injected seam.
   */
  async save(api: SenseiApi, projectId: string): Promise<boolean> {
    if (!projectId) return false;
    this.saveStatus = 'saving';
    this.saveError = null;
    try {
      const res = await api.tryUpdateProject(projectId, this.toPatch());
      if (res.ok) {
        this.saveStatus = 'saved';
        return true;
      }
      this.saveStatus = 'error';
      this.saveError = res.error.message;
      return false;
    } catch (e) {
      this.saveStatus = 'error';
      this.saveError = e instanceof Error ? e.message : String(e);
      return false;
    }
  }
}
