// Relay primitives — the class vocabulary of COMPONENTS.md §3a (RoleMark; Pill and
// Tag are rokkit's Pill, #410). Pure data: the component specs render the fixtures and pin the
// output; this module is what they draw, so a wrong tone, radius or type stop
// fails here on data instead of inside a DOM snapshot.
//
// Every class string is literal on purpose. UnoCSS scans source text, so a
// class assembled from a template (`bg-${tone}-soft`) never reaches the
// generated CSS; and the mockup's tint() is written out as `color-mix` because
// UnoCSS emits no opacity modifier for a named token (`border-success/35` is
// not generated).

import type { RelayRole } from '$lib/relay-api.js';

// ── RoleMark (§3a) ──────────────────────────────────────────────────────────

export type RoleStatus = 'active' | 'idle' | 'not_responding';

export const ROLE_STATUSES: RoleStatus[] = ['active', 'idle', 'not_responding'];

const ROLE_LABEL: Record<RelayRole, string> = {
  coordinator: 'Coordinator',
  contributor: 'Contributor',
};

/** Status tone and its word: colour is never the only signal (the title says it). */
const STATUS: Record<RoleStatus, { tone: string; label: string }> = {
  active: { tone: 'success', label: 'Active' },
  idle: { tone: 'ink-mute', label: 'Idle' },
  not_responding: { tone: 'warning', label: 'Not responding' },
};

const MARK_FRAME = 'shrink-0 inline-flex items-center justify-center rounded-full border border-solid';

/** A lane mark: 24px, coordinator accent-tinted at 20%, contributor on paper. */
const LANE_FRAME: Record<RelayRole, string> = {
  coordinator: 'w-6 h-6 border-paper-edge bg-[color-mix(in_oklab,var(--accent)_20%,var(--paper))]',
  contributor: 'w-6 h-6 border-paper-edge bg-paper',
};

const LANE_GLYPH = 'text-xs text-accent';

/** A crew mark: 30px, filled with the status tone at 14%, bordered at 45%. */
const CREW_FRAME: Record<RoleStatus, string> = {
  active:
    'w-[30px] h-[30px] bg-[color-mix(in_oklab,var(--success)_14%,var(--paper))] border-[color-mix(in_oklab,var(--success)_45%,var(--paper))]',
  idle: 'w-[30px] h-[30px] bg-[color-mix(in_oklab,var(--ink-mute)_14%,var(--paper))] border-[color-mix(in_oklab,var(--ink-mute)_45%,var(--paper))]',
  not_responding:
    'w-[30px] h-[30px] bg-[color-mix(in_oklab,var(--warning)_14%,var(--paper))] border-[color-mix(in_oklab,var(--warning)_45%,var(--paper))]',
};

const CREW_GLYPH: Record<RoleStatus, string> = {
  active: 'text-sm text-success',
  idle: 'text-sm text-ink-mute',
  not_responding: 'text-sm text-warning',
};

export interface RoleMarkView {
  frame: string;
  glyph: string;
  title: string;
}

/**
 * A lane mark (`size` 24) carries the role; a crew mark (`size` 30) carries the
 * status, in its fill, its border, its glyph and its title.
 */
export function roleMarkClass(role: RelayRole, status: RoleStatus, size: 24 | 30): RoleMarkView {
  if (size === 30) {
    return {
      frame: `${MARK_FRAME} ${CREW_FRAME[status]}`,
      glyph: CREW_GLYPH[status],
      title: `${ROLE_LABEL[role]} · ${STATUS[status].label}`,
    };
  }
  return {
    frame: `${MARK_FRAME} ${LANE_FRAME[role]}`,
    glyph: LANE_GLYPH,
    title: ROLE_LABEL[role],
  };
}

// ── Now rows (§3b) ───────────────────────────────────────────────────────────

/** A lane or completed row (mockup `laneRow`): full width, wraps on a phone, pad 10/14, gap 4×12. */
export const NOW_ROW =
  'w-full flex flex-wrap items-center gap-x-3 gap-y-1 py-2.5 px-3.5 text-ink no-underline text-left';

/** The card a Now list's rows sit in: 1px edge, radius 12, paper; rows split by a mute hairline. */
export const NOW_CARD = 'm-0 p-0 list-none flex flex-col border border-solid border-paper-edge rounded-xl bg-paper overflow-hidden';

/** A row inside a NOW_CARD. */
export const NOW_CARD_ROW = 'border-t border-solid border-paper-mute first:border-t-0';
