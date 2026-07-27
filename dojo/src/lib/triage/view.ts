// Pure presentation helpers for the maintainer-console triage screens (R9).
//
// All functions are side-effect-free (data in → display value out) so they unit
// test without a DOM or a live backend, and so the screens stay declarative.
// Colour is expressed as a token utility CLASS name (text-accent / text-success
// / …) per the console design system — never a raw hex/oklch.

import type { TriageRow } from '../triage-data';

// ── artifact kind → label + kanji (mockup DOJO_TYPE + a per-kind glyph) ───────

const KIND_LABELS: Record<string, string> = {
	guard: 'Guard',
	pattern: 'Pattern',
	principle: 'Principle',
	prompt: 'Prompt',
	skill: 'Skill',
	agent: 'Agent',
	persona: 'Persona'
};

/** Human label for an artifact kind; falls back to a capitalised raw value. */
export function kindLabel(kind: string): string {
	return KIND_LABELS[kind] ?? (kind ? kind[0].toUpperCase() + kind.slice(1) : 'Artifact');
}

const KIND_KANJI: Record<string, string> = {
	guard: '守',
	pattern: '紋',
	principle: '則',
	prompt: '問',
	skill: '技',
	agent: '士',
	persona: '面'
};

/** A kanji glyph for the kind's row marker (defaults to the generic 芽 "seed"). */
export function kindKanji(kind: string): string {
	return KIND_KANJI[kind] ?? '芽';
}

// ── owner scope (opaque JSON) → a readable label ─────────────────────────────

/**
 * Best-effort label for a triage row's `owner_scope`. The store returns the
 * artifact's `scope` JSON verbatim, whose shape isn't guaranteed — read the
 * common fields defensively and fall back to a compact JSON string, never throw.
 */
export function scopeLabel(scope: unknown): string {
	if (scope == null) return 'Unscoped';
	if (typeof scope === 'string') return scope;
	if (typeof scope === 'object') {
		const o = scope as Record<string, unknown>;
		const label = o.label ?? o.name ?? o.scope ?? o.key;
		if (typeof label === 'string' && label) return label;
		const kind = typeof o.kind === 'string' ? o.kind : undefined;
		const value = o.value ?? o.id ?? o.team ?? o.stack ?? o.client;
		if (kind && typeof value === 'string') return `${kind} · ${value}`;
		if (kind) return kind;
	}
	try {
		return JSON.stringify(scope);
	} catch {
		return 'Scope';
	}
}

// ── confidence / similarity → text-token class ───────────────────────────────

/**
 * Token utility CLASS for a confidence value (mockup thresholds: ≥.85 success,
 * ≥.70 accent, else warning). `null` reads as unscored (muted).
 */
export function confidenceToneClass(v: number | null): string {
	if (v == null) return 'text-ink-mute';
	if (v >= 0.85) return 'text-success';
	if (v >= 0.7) return 'text-accent';
	return 'text-warning';
}

/** Confidence as an integer 0..100 for display (`—` when unscored). */
export function confidencePct(v: number | null): string {
	return v == null ? '—' : String(Math.round(v * 100));
}

/** Confidence bar width as a CSS percentage (0% when unscored). */
export function confidenceWidth(v: number | null): string {
	return `${Math.round((v ?? 0) * 100)}%`;
}

/**
 * Auto-dedupe band for a similarity (mockup: ≥0.90 merges automatically,
 * 0.75–0.90 is flagged for confirmation, below that is a weak match). Drives the
 * similarity chip tone on the candidate screen.
 */
export function similarityBand(v: number | null): 'none' | 'weak' | 'flagged' | 'merge' {
	if (v == null) return 'none';
	if (v >= 0.9) return 'merge';
	if (v >= 0.75) return 'flagged';
	return 'weak';
}

export function similarityToneClass(v: number | null): string {
	switch (similarityBand(v)) {
		case 'merge':
			return 'text-accent';
		case 'flagged':
			return 'text-warning';
		default:
			return 'text-ink-mute';
	}
}

// ── age (created_at ISO → compact relative) ──────────────────────────────────

/**
 * Compact "how long ago" for an ISO timestamp against `now` (default: current
 * time). Returns values like `3d`, `5h`, `12m`, `just now`. Injectable `now`
 * keeps it deterministic under test.
 */
export function relativeAge(iso: string, now: Date = new Date()): string {
	const then = new Date(iso).getTime();
	if (Number.isNaN(then)) return '';
	const secs = Math.max(0, Math.floor((now.getTime() - then) / 1000));
	if (secs < 60) return 'just now';
	const mins = Math.floor(secs / 60);
	if (mins < 60) return `${mins}m`;
	const hours = Math.floor(mins / 60);
	if (hours < 24) return `${hours}h`;
	const days = Math.floor(hours / 24);
	return `${days}d`;
}

// ── grouping (mockup DojoTriage: group by scope, rank by confidence) ──────────

export interface TriageGroup {
	scope: string;
	rows: TriageRow[];
}

/**
 * Group open rows by their owner-scope label, ranked strongest-first within each
 * group and across groups (highest confidence, then newest — a null confidence
 * sorts last). Pure: same input → same output.
 */
export function groupByScope(rows: TriageRow[]): TriageGroup[] {
	const byScope = new Map<string, TriageRow[]>();
	for (const r of rows) {
		const key = scopeLabel(r.owner_scope);
		const bucket = byScope.get(key);
		if (bucket) bucket.push(r);
		else byScope.set(key, [r]);
	}
	const rank = (a: TriageRow, b: TriageRow) => {
		const ca = a.confidence ?? -1;
		const cb = b.confidence ?? -1;
		if (cb !== ca) return cb - ca;
		return b.created_at.localeCompare(a.created_at);
	};
	const groups: TriageGroup[] = [...byScope.entries()].map(([scope, groupRows]) => ({
		scope,
		rows: [...groupRows].sort(rank)
	}));
	groups.sort((a, b) => rank(a.rows[0], b.rows[0]));
	return groups;
}
