// Developer / personal-console catalog (Chunk 4 of the Dōjō IA rebuild; mockup
// docs/mockups/Sensei/lib/dojo/dojo-developer.jsx — `DEV_ROLE_BY_KIND`,
// `DEV_FOLLOWS`, and the `mine` / `items` literals inside DojoDevContributions /
// DojoDevDownstream).
//
// The developer console is the individual contributor's read-mostly seat: one
// login, every Dōjō they belong to. It has three surfaces —
//   • My teams          — every membership + what each one follows.
//   • My contributions  — lessons sent upstream + status per destination.
//   • For me            — approved teachings distributed down to them.
//
// This module is DATA only — pure typed constants, no DOM, no fetch — so the
// three sections are driven from one source of truth and the grouping/count math
// in `developer-view.ts` unit-tests against it. `/v1` wiring to real
// `dojo.memberships` / `dojo.shared_rules` is deferred (see the plan's Chunk 4),
// so contributions + downstream are local sample rows for now. "My teams" is
// driven by the caller's REAL memberships (the layout's `data.memberships`); this
// catalog supplies only the per-membership "role" + "what it follows" overlay the
// live membership row doesn't yet carry, keyed by the membership id.

import type { OrgKind } from './dojo-data';

// ── My teams: per-membership overlay ─────────────────────────────────────────
// The role a contributor holds in a Dōjō of a given kind (mockup
// `DEV_ROLE_BY_KIND`, keyed there on lowercase kind). Mapped onto the app's
// PascalCase `OrgKind`. Client memberships anonymize a contributor's sends.
export const DEV_ROLE_BY_KIND: Record<OrgKind, string> = {
	Organization: 'Contributor',
	Client: 'Contributor · anonymized',
	Community: 'Member',
	Personal: 'Owner'
};

// What each membership follows — the areas / packages a contributor draws from
// there (mockup `DEV_FOLLOWS`, keyed on the membership id). A membership without
// an entry falls back to a neutral label in the view helper.
export const DEV_FOLLOWS: Record<string, string> = {
	acme: 'Web · Auth · Payments',
	globex: 'lumen-auth · billing',
	initech: 'initech-portal',
	rustco: 'rust · axum · sqlx',
	self: 'everything (private)'
};

// ── My contributions: upstream sends + status ────────────────────────────────

/** Where each contribution stands once it lands in a Dōjō's triage queue. */
export type ContributionStatus = 'approved' | 'pending' | 'declined';

/** A lesson the contributor sent upstream, and how it fared per destination
 *  (mockup DojoDevContributions `mine`). `client` marks a client-scoped send —
 *  automatically anonymized. `kanji` is the topic glyph; `scope` is the ladder
 *  rung it applies at; `note` is the maintainer's outcome line. */
export interface Contribution {
	id: string;
	kanji: string;
	title: string;
	/** the Dōjō it was sent to (the destination name). */
	dest: string;
	scope: string;
	status: ContributionStatus;
	/** relative age, e.g. "2d" / "6h" (display only). */
	when: string;
	note: string;
	client: boolean;
}

/** The contributor's upstream sends, newest-intent first (mirrors the mockup). */
export const CONTRIBUTIONS: readonly Contribution[] = [
	{
		id: 'c-adapter',
		kanji: '紋',
		title: 'Adapter wraps a third-party SDK behind a trait',
		dest: 'Acme Corp',
		scope: 'Stack · Rust',
		status: 'approved',
		when: '2d',
		note: 'published · +7pp FTR',
		client: false
	},
	{
		id: 'c-runes',
		kanji: '直',
		title: '`let` → `$state(...)` in Svelte 5 components',
		dest: 'Rust Guild',
		scope: 'Stack · Svelte',
		status: 'pending',
		when: '6h',
		note: 'in triage · owner Sven K.',
		client: false
	},
	{
		id: 'c-webhook',
		kanji: '盾',
		title: 'Verify webhook signature before parsing',
		dest: 'Globex',
		scope: 'Client · anonymized',
		status: 'approved',
		when: '1d',
		note: 'anonymized · shared safely',
		client: true
	},
	{
		id: 'c-persona',
		kanji: '問',
		title: 'Persona: integration-test author for auth',
		dest: 'Acme Corp',
		scope: 'Stack · React',
		status: 'declined',
		when: '3d',
		note: 'merged into an existing persona',
		client: false
	}
];

/** Lifetime reach shown in the contributions header (mockup: "612 devs helped"). */
export const CONTRIB_DEVS_HELPED = 612;

// ── For me: downstream (approved teachings distributed to me) ─────────────────

/** The kind of teaching distributed down — a guard, a pattern, or a skill. */
export type DownstreamKind = 'guard' | 'pattern' | 'skill';

/** An approved teaching distributed down to the contributor (mockup
 *  DojoDevDownstream `items`). `adopted` marks one already taken up (vs. new);
 *  `from` is the originating Dōjō; `scope` is the ladder rung it arrived at. */
export interface Downstream {
	id: string;
	kanji: string;
	title: string;
	from: string;
	scope: string;
	when: string;
	adopted: boolean;
	kind: DownstreamKind;
}

/** The teachings distributed down to the contributor (mirrors the mockup). */
export const DOWNSTREAM: readonly Downstream[] = [
	{
		id: 'd-refresh',
		kanji: '守',
		title: 'Never log refresh tokens, even at debug level',
		from: 'Acme Corp',
		scope: 'Company',
		when: '8m',
		adopted: false,
		kind: 'guard'
	},
	{
		id: 'd-idempotency',
		kanji: '紋',
		title: 'Idempotency key on money-moving mutations',
		from: 'Acme Corp',
		scope: 'Team · Payments',
		when: '4h',
		adopted: true,
		kind: 'pattern'
	},
	{
		id: 'd-queryplan',
		kanji: '技',
		title: 'Skill: explain a slow query plan',
		from: 'Rust Guild',
		scope: 'Stack · Postgres',
		when: '1d',
		adopted: false,
		kind: 'skill'
	}
];
