// Workspace · About — Dōjō binding chip (R3 workspace→Dōjō auto-bind).
//
// The daemon owns the inference (`GET /api/workspaces/{id}/dojo-suggestion`) and
// the confirm write (`POST …/dojo-binding`). This module only:
//   - resolves the current binding into a view model (confirmed | inferred |
//     empty) from the workspace + its suggestion + the membership list, and
//   - owns the Confirm action's pending / error / client-side-confirmed state
// so the `+page.svelte` stays a pure template.
//
// Wire-API-wins: the confirmed row is resolved from two signals the wire
// carries — `workspaces.dojo_id` (the primary; the `/api/workspaces` list endpoint
// serializes it) AND each membership's `bound_workspaces` strip (a secondary
// signal, robust if a caller's workspace payload omits `dojo_id`). `kind` is a
// free wire string, so it degrades to a fallback glyph rather than being
// dropped — mirroring the connections module.

import type { SenseiApi } from '$lib/api.js';
import type { DojoBindingSuggestion, DojoMembership } from '$lib/types.js';
// Reuse the connections module's kind→kanji map rather than duplicating it —
// the four membership kinds and their glyphs are owned there (DRY).
import { kindPill } from '../../../../(observatory)/dojo/connections/dojo-connections-state.svelte.js';

/** Named-token utility classes for a binding-state chip, plus its label. */
export interface BindingChip {
  bg: string;
  text: string;
  label: string;
}

/** The confirmed binding — a workspace bound to a known membership. */
export interface ConfirmedBinding {
  state: 'confirmed';
  kanji: string;
  /** The membership's tenant key (mono meta line). */
  tenantKey: string;
  /** The membership's Dōjō url (mono meta line). */
  dojoUrl: string;
}

/** An inferred (not-yet-confirmed) binding awaiting the user's Confirm. */
export interface InferredBinding {
  state: 'inferred';
  kanji: string;
  membershipId: string;
  matchedSlug: string;
  tenantKey: string;
  dojoUrl: string;
}

/** No binding and nothing inferred — findings stay local. */
export interface EmptyBinding {
  state: 'empty';
}

/** The binding could not be read — shown as an error, never as "no binding". */
export interface ErrorBinding {
  state: 'error';
  message: string;
}

export type BindingView = ConfirmedBinding | InferredBinding | EmptyBinding | ErrorBinding;

/** The minimal workspace shape this module reads. `dojo_id` is optional so any
 *  workspace payload works; when present it is the primary confirmed signal, else
 *  the confirmed row is resolved from `bound_workspaces`. */
export interface BindableWorkspace {
  id: string;
  dojo_id?: string | null;
}

/** Pure: kind → kanji glyph, reusing the connections module's map. */
export function bindingKanji(kind: string): string {
  return kindPill(kind).kanji;
}

/** Pure: the confirmed-state chip — success tone, sentence-case label. */
export function confirmedChip(): BindingChip {
  return { bg: 'bg-success-soft', text: 'text-on-success-soft', label: 'confirmed' };
}

/** Pure: the inferred-state chip — warning tone, sentence-case label. */
export function inferredChip(): BindingChip {
  return { bg: 'bg-warning-soft', text: 'text-on-warning-soft', label: 'inferred' };
}

/** Pure: find the membership this workspace is bound to, or null. Confirmed when
 *  either `workspaces.dojo_id` points at a membership OR the workspace appears in a
 *  membership's `bound_workspaces` strip — whichever the wire carries. */
export function boundMembership(
  workspace: BindableWorkspace,
  memberships: DojoMembership[],
): DojoMembership | null {
  const dojoId = workspace.dojo_id ?? null;
  if (dojoId) {
    const byId = memberships.find((m) => m.id === dojoId);
    if (byId) return byId;
  }
  const byBound = memberships.find((m) =>
    m.bound_workspaces.some((b) => b.id === workspace.id),
  );
  return byBound ?? null;
}

/** Pure: resolve the binding view model. Confirmed wins over inferred (a bound
 *  workspace never shows the inferred prompt); inferred wins over empty. */
export function resolveBinding(
  workspace: BindableWorkspace,
  suggestion: DojoBindingSuggestion | null,
  memberships: DojoMembership[],
  error: string | null = null,
): BindingView {
  if (error) return { state: 'error', message: error };
  const bound = boundMembership(workspace, memberships);
  if (bound) {
    return {
      state: 'confirmed',
      kanji: bindingKanji(bound.kind),
      tenantKey: bound.tenant_key,
      dojoUrl: bound.dojo_url,
    };
  }
  if (suggestion) {
    return {
      state: 'inferred',
      kanji: bindingKanji(suggestion.kind),
      membershipId: suggestion.membership_id,
      matchedSlug: suggestion.matched_slug,
      tenantKey: suggestion.tenant_key,
      dojoUrl: suggestion.dojo_url,
    };
  }
  return { state: 'empty' };
}

/**
 * Owns the Confirm action for the inferred binding. `pending` gates the button,
 * `error` carries a failed round-trip's message, and `confirmed` is the
 * client-side override that replaces the inferred row with the confirmed row on
 * success (the load's data is stale until the next navigation). The api client
 * is passed into `confirm` as the seam so the state is unit-testable with a
 * hand-rolled mock and never reaches for a global.
 */
export class BindingAction {
  pending = $state(false);
  error = $state<string | null>(null);
  /** The confirmed view to render after a successful confirm, or null before. */
  confirmed = $state<ConfirmedBinding | null>(null);

  /**
   * Confirm the inferred binding. Builds the confirmed view from the inferred
   * one so the row swaps without a reload. Resolves `true` on success (the page
   * may also `invalidateAll()`), `false` on failure with `error` set.
   */
  async confirm(api: SenseiApi, workspaceId: string, inferred: InferredBinding): Promise<boolean> {
    this.pending = true;
    this.error = null;
    const res = await api.bindWorkspaceDojo(workspaceId, inferred.membershipId);
    this.pending = false;
    if (res.ok) {
      this.confirmed = {
        state: 'confirmed',
        kanji: inferred.kanji,
        tenantKey: inferred.tenantKey,
        dojoUrl: inferred.dojoUrl,
      };
      return true;
    }
    this.error = res.error.message;
    return false;
  }
}
