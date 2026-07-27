// dojo kit — shared vocabulary maps (ported from dojo2-kit.jsx K2_CLASS /
// K2_PHASE / K2_ROLE / K2_KIND). This is the ONE place tones are decided.
//
// Ported faithfully but token-native: the mockup stored raw `var(--*)` strings;
// here each tone is a named-token utility CLASS (`text-accent`, `bg-accent-soft`,
// …) so components stay theme-free and never hand-code a color — the same idiom
// as `triage-view.ts`'s `confidenceToneClass`. `text`/`fill` tint foreground,
// `soft` fills a pill background, `edge` colors its hairline border.

export interface ClassTone {
	/** Kanji brand glyph for the classification. */
	kanji: string;
	/** Human label. */
	label: string;
	/** Foreground text token class. */
	text: string;
	/** Background fill token class (soft tint). */
	soft: string;
	/** Border token class (edge tint). */
	edge: string;
	/** Solid-fill background token class (for dots). */
	fill: string;
}

/** Project classification — company · client · personal · community. */
export const K2_CLASS: Record<string, ClassTone> = {
	company: {
		kanji: '社',
		label: 'company',
		text: 'text-ink-soft',
		soft: 'bg-paper-mute',
		edge: 'border-paper-edge',
		fill: 'bg-ink-soft'
	},
	client: {
		kanji: '客',
		label: 'client',
		text: 'text-accent',
		soft: 'bg-accent-soft',
		edge: 'border-accent-soft',
		fill: 'bg-accent'
	},
	personal: {
		kanji: '己',
		label: 'personal',
		text: 'text-ink-mute',
		soft: 'bg-paper-mute',
		edge: 'border-paper-edge',
		fill: 'bg-ink-mute'
	},
	community: {
		kanji: '群',
		label: 'community',
		text: 'text-success',
		soft: 'bg-success-soft',
		edge: 'border-success-soft',
		fill: 'bg-success'
	}
};

export interface PhaseTone {
	kanji: string;
	label: string;
	/** Foreground text token class for the label. */
	text: string;
	/** Solid-fill token class for the lit track dots. */
	dot: string;
	/** How many of the 3 track steps are lit (1..3). */
	step: number;
}

/** Adoption phase — watch → notice → adopt (a 3-step track). */
export const K2_PHASE: Record<string, PhaseTone> = {
	watch: { kanji: '観', label: 'watch', text: 'text-ink-mute', dot: 'bg-ink-mute', step: 1 },
	notice: { kanji: '察', label: 'notice', text: 'text-warning', dot: 'bg-warning', step: 2 },
	adopt: { kanji: '覚', label: 'adopt', text: 'text-success', dot: 'bg-success', step: 3 }
};

export interface RoleTone {
	kanji: string;
	label: string;
	/** Solar icon name for the role tag. */
	icon: string;
}

/** Additive membership role — developer · maintainer · lead · admin. */
export const K2_ROLE: Record<string, RoleTone> = {
	developer: { kanji: '士', label: 'developer', icon: 'code' },
	maintainer: { kanji: '掟', label: 'maintainer', icon: 'settings' },
	lead: { kanji: '客', label: 'lead', icon: 'user' },
	admin: { kanji: '任', label: 'admin', icon: 'shield' }
};

export interface KindTone {
	kanji: string;
	/** Foreground text token class for the org kind glyph. */
	text: string;
}

/** Dōjō kind — employer · client · community · personal. */
export const K2_KIND: Record<string, KindTone> = {
	employer: { kanji: '社', text: 'text-ink-soft' },
	client: { kanji: '客', text: 'text-accent' },
	community: { kanji: '群', text: 'text-success' },
	personal: { kanji: '己', text: 'text-ink-mute' }
};

/** Look up a classification tone, defaulting to `company`. */
export function classTone(kind: string | null | undefined): ClassTone {
	return (kind && K2_CLASS[kind]) || K2_CLASS.company;
}

/** Look up a phase tone, defaulting to `watch`. */
export function phaseTone(phase: string | null | undefined): PhaseTone {
	return (phase && K2_PHASE[phase]) || K2_PHASE.watch;
}

/** Look up a role tone, or `undefined` when the role is unknown. */
export function roleTone(role: string | null | undefined): RoleTone | undefined {
	return role ? K2_ROLE[role] : undefined;
}

/** Look up a dōjō-kind tone, defaulting to `employer`. */
export function kindTone(kind: string | null | undefined): KindTone {
	return (kind && K2_KIND[kind]) || K2_KIND.employer;
}

// ── Plan task states (ported from dojo2-kit.jsx K2_NODE) ──────────────────────

import type { KitTaskState } from './types';

export interface NodeTone {
	/** Solar icon name. */
	icon: string;
	/** Human label. */
	label: string;
	/** Foreground text token class. */
	text: string;
	/** Solid-fill token class (pips · dots). */
	fill: string;
	/** Soft-tint background token class (node background). */
	soft: string;
	/** Border token class. */
	edge: string;
	/** Render the node border dashed (pending). */
	dashed?: boolean;
}

/** The seven task-node tones — one place decides every plan-state color. */
export const K2_NODE: Record<KitTaskState, NodeTone> = {
	done: { icon: 'check-circle', label: 'done', text: 'text-ink-mute', fill: 'bg-ink-mute', soft: 'bg-paper-mute', edge: 'border-paper-edge' },
	active: { icon: 'play-circle', label: 'active', text: 'text-success', fill: 'bg-success', soft: 'bg-success-soft', edge: 'border-success-soft' },
	needs_review: { icon: 'shield-warning', label: 'needs review', text: 'text-accent', fill: 'bg-accent', soft: 'bg-accent-soft', edge: 'border-accent-soft' },
	blocked: { icon: 'lock-keyhole-minimalistic', label: 'blocked', text: 'text-warning', fill: 'bg-warning', soft: 'bg-warning-soft', edge: 'border-warning-soft' },
	failed: { icon: 'close-circle', label: 'failed', text: 'text-danger', fill: 'bg-danger', soft: 'bg-danger-soft', edge: 'border-danger-edge' },
	skipped: { icon: 'forward', label: 'skipped', text: 'text-ink-faint', fill: 'bg-ink-faint', soft: 'bg-paper-mute', edge: 'border-paper-edge' },
	pending: { icon: 'clock-circle', label: 'pending', text: 'text-ink-faint', fill: 'bg-ink-faint', soft: 'bg-transparent', edge: 'border-paper-edge', dashed: true }
};

/** Legacy authored states → the canonical seven (older plans used these). */
export const STATE_ALIAS: Record<string, KitTaskState> = {
	queued: 'pending',
	running: 'active',
	gate: 'needs_review'
};

/** Normalize any state string (incl. legacy aliases) to a `KitTaskState`,
 * defaulting to `pending`. */
export function taskState(state: string | null | undefined): KitTaskState {
	if (!state) return 'pending';
	return STATE_ALIAS[state] ?? (state in K2_NODE ? (state as KitTaskState) : 'pending');
}

/** Look up a task-node tone, defaulting to `pending`. */
export function nodeTone(state: string | null | undefined): NodeTone {
	return K2_NODE[taskState(state)];
}

// ── Inbox run status (the /you inbox row badge) ──────────────────────────────

export interface StatusTone {
	label: string;
	/** Foreground text token class. */
	text: string;
	/** Solid-fill token class (the status dot). */
	fill: string;
}

/** Run status → badge tone for the inbox row (see relay-map `inboxStatus`). */
export const K2_STATUS: Record<string, StatusTone> = {
	running: { label: 'running', text: 'text-success', fill: 'bg-success' },
	waiting: { label: 'waiting', text: 'text-ink-soft', fill: 'bg-ink-soft' },
	stalled: { label: 'stalled', text: 'text-warning', fill: 'bg-warning' },
	blocked: { label: 'blocked', text: 'text-warning', fill: 'bg-warning' },
	failed: { label: 'failed', text: 'text-danger', fill: 'bg-danger' },
	done: { label: 'done', text: 'text-ink-mute', fill: 'bg-ink-mute' }
};

/** Look up a run-status tone, defaulting to a neutral waiting-like tone. */
export function statusTone(status: string | null | undefined): StatusTone {
	return (status && K2_STATUS[status]) || { label: status ?? '', text: 'text-ink-mute', fill: 'bg-ink-mute' };
}
