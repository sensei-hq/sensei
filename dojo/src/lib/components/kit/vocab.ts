// dojo2 kit — shared vocabulary maps (ported from dojo2-kit.jsx K2_CLASS /
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
