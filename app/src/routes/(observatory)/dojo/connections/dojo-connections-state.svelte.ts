// Observatory · Dōjō connections — derivations, status→token mapping and copy.
// The daemon owns memberships
// (`crates/senseid/src/dojo/memberships.rs`); this module only maps the wire
//
// NOTE: the sync-status DERIVATION this pane displays does not exist yet. No
// code writes `dojo_memberships.last_heartbeat_at`, and the only `sync_status`
// setter has no callers, so every row reads `authenticating` forever and the
// `healthy` count below is permanently 0 ("1 connection · 0 healthy"). The
// mapping is kept because the vocabulary is right; only the producer is
// missing. See `database/ddl/enum/dojo/sync_status.ddl`.
// discriminators (sync_status, kind) to named-token chip classes + copy, so the
// `+page.svelte` stays a pure template. There is no add-a-connection form:
// signing in is joining (#236 step 4e).
//
// Wire-API-wins: `sync_status` and `kind` come back as free strings, so an
// unexpected value degrades to muted ink rather than being dropped — mirroring
// the traceability state module's discipline.

import type { DojoBoundWorkspace, DojoMembership } from '$lib/types.js';

/** Semantic tone a sync status maps to. `ink` = an unexpected status we still
 *  surface rather than drop (never a status colour). */
export type SyncTone = 'success' | 'warning' | 'danger' | 'info' | 'ink';

/** Named-token utility classes for a chip, plus the label to print. */
export interface ChipClasses {
  bg: string;
  text: string;
  label: string;
}

/** A kind chip — kanji glyph + label + named-token classes. */
export interface KindPill {
  kanji: string;
  label: string;
  bg: string;
  text: string;
}

/** Pure: sync status → semantic tone. `healthy` reads success, `stale` warning,
 *  `error` danger, `authenticating` info; anything else degrades to muted ink so
 *  an unexpected wire value is still shown, never hidden. */
export function syncTone(status: string): SyncTone {
  switch (status) {
    case 'healthy':
      return 'success';
    case 'stale':
      return 'warning';
    case 'error':
      return 'danger';
    case 'authenticating':
      return 'info';
    default:
      return 'ink';
  }
}

/** Pure: named-token chip classes + label for a sync status. The label is the
 *  raw wire status (honest — a `healthy` chip never claims "connected" when the
 *  heartbeat says otherwise). The lookup IS the rendering rule, so it lives with
 *  the domain rather than in the component. */
export function syncChip(status: string): ChipClasses {
  switch (syncTone(status)) {
    case 'success':
      return { bg: 'bg-success-soft', text: 'text-success', label: status };
    case 'warning':
      return { bg: 'bg-warning-soft', text: 'text-warning', label: status };
    case 'danger':
      return { bg: 'bg-danger-soft', text: 'text-danger', label: status };
    case 'info':
      return { bg: 'bg-info-soft', text: 'text-info', label: status };
    default:
      return { bg: 'bg-paper-mute', text: 'text-ink-soft', label: status };
  }
}

/** Pure: kind → kanji glyph + label + chip classes. `client` is given the accent
 *  treatment so client-precedence stays visible (spec "wrong gate": a client kind
 *  that isn't distinguished hides the precedence story); the rest read muted. */
export function kindPill(kind: string): KindPill {
  switch (kind) {
    case 'employer':
      return { kanji: '社', label: 'employer', bg: 'bg-paper-mute', text: 'text-ink-soft' };
    case 'client':
      return { kanji: '客', label: 'client', bg: 'bg-accent-soft', text: 'text-accent' };
    case 'community':
      return { kanji: '群', label: 'community', bg: 'bg-paper-mute', text: 'text-ink-soft' };
    case 'personal':
      return { kanji: '己', label: 'personal', bg: 'bg-paper-mute', text: 'text-ink-soft' };
    default:
      return { kanji: '結', label: kind || 'unknown', bg: 'bg-paper-mute', text: 'text-ink-soft' };
  }
}

/** Pure: "3 workspaces bound" / "1 workspace bound" / "no workspaces bound". */
export function boundWorkspacesSummary(workspaces: DojoBoundWorkspace[]): string {
  const n = workspaces.length;
  if (n === 0) return 'no workspaces bound';
  return `${n} ${n === 1 ? 'workspace' : 'workspaces'} bound`;
}

/** Pure: the list header line, e.g. "3 connections · 2 healthy." Sentence case,
 *  no exclamation — voice charter. */
export function connectionsHeadline(connections: DojoMembership[]): string {
  const n = connections.length;
  const healthy = connections.filter((c) => c.sync_status === 'healthy').length;
  const conns = `${n} ${n === 1 ? 'connection' : 'connections'}`;
  return `${conns} · ${healthy} healthy.`;
}
