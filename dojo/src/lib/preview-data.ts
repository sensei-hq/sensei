// Effective-constitution preview catalog — the sample projects and their scope
// ladders (Chunk 3 of the Dōjō IA rebuild; mockup dojo-preview.jsx `PV_PROJECTS`,
// `pvLadder`). Answers the question a developer actually asks before the first
// commit: "what rules govern THIS project?" — by resolving the whole ladder
// (Company → Client → Personal → Project → Stack) for one project.
//
// This module is DATA only — pure typed constants + one pure ladder builder, no
// DOM, no fetch — so the resolution engine in `preview-view.ts` can be unit-tested
// against it and the screen renders from one source of truth. The rung levels
// reuse the library ladder vocabulary (library-data `LibLevelId` covers Org/Team/
// Project/Stack; preview adds the Company/Client/Personal engagement rungs on top,
// since a project is resolved through the engagement ladder, not the authoring
// one). `/v1` wiring to real resolved rules for a real project is deferred (see
// the plan's Chunk 3).

/** A rung of the ladder, broad → specific. Company/Client are the paid
 *  engagement rungs; Personal/Stack are free; Project is per-repo. */
export type ScopeId = 'company' | 'client' | 'personal' | 'project' | 'stack';

/** How a project resolves — its lifecycle. Drives which rungs switch on:
 *  `company` = your employer's own product (no Client rung); `client` = another
 *  org's engagement (Client + Company both apply); `personal` = yours alone
 *  (free personal ladder only). Mirrors the mockup's `kind`. */
export type ProjectKind = 'company' | 'client' | 'personal';

/** One rule on a rung. `topic` is the conflict axis (two rungs with the same
 *  topic collide); `hard` marks a non-negotiable (★) that locks — no narrower
 *  scope can relax it; `relaxOf` names the topic this rule tries to relax (a
 *  narrower scope loosening a broader bar) — it wins only if the broader rule on
 *  that topic is NOT hard. `src` tags a compliance-sourced rule (法 chip). */
export interface PreviewRule {
	/** the conflict axis — rules sharing a topic across rungs collide. */
	topic: string;
	/** the rule copy. */
	text: string;
	/** a non-negotiable (★) — locks the topic; a narrower scope can't relax it. */
	hard: boolean;
	/** when set, this rule attempts to relax the broader rule on the same topic. */
	relaxOf?: boolean;
	/** provenance tag; `compliance` renders the 法 chip. */
	src?: 'compliance';
}

/** A rung: an ordered scope with a kanji, a display name, a scope label (whose
 *  rules these are), a caption, whether it is free, optional wired `checkers`
 *  (stack rungs), and its rules. */
export interface Rung {
	id: string;
	scope: ScopeId;
	kanji: string;
	name: string;
	label: string;
	caption: string;
	free: boolean;
	checkers: readonly string[];
	rules: readonly PreviewRule[];
}

/** A sample project the preview can resolve — one per lifecycle case. */
export interface PreviewProject {
	id: string;
	kanji: string;
	name: string;
	repo: string;
	kind: ProjectKind;
	/** the bound Dōjō, or null for a purely personal project. */
	dojo: string | null;
	team: string | null;
	stack: readonly string[];
	/** for an agency monorepo: the multiple clients it serves. */
	clients?: readonly string[];
	/** the plain-language "why this classification" explainer. */
	why: string;
}

/** Broad → specific order of the ladder. Index = specificity rank (lower = broader).
 *  The single source of truth both the ladder builder and the resolver use so
 *  "most specific wins" is a comparison of ladder position, never a magic number. */
export const SCOPE_ORDER: readonly ScopeId[] = [
	'company',
	'client',
	'personal',
	'project',
	'stack'
] as const;

/** The specificity rank of a scope (its index in SCOPE_ORDER). Broader = smaller. */
export function scopeRank(scope: ScopeId): number {
	return SCOPE_ORDER.indexOf(scope);
}

/** The four sample projects — one per lifecycle (company / client / personal /
 *  agency-monorepo). Mirrors the mockup `PV_PROJECTS`. */
export const PV_PROJECTS: readonly PreviewProject[] = [
	{
		id: 'auth',
		kanji: '件',
		name: 'lumen-auth',
		repo: 'acme/lumen-auth',
		kind: 'company',
		dojo: 'Acme Corp',
		team: 'Payments',
		stack: ['React', 'TypeScript'],
		why: "This repo belongs to Acme — your employer's own product. No client rung: Company · Personal · Project · Stack."
	},
	{
		id: 'globex',
		kanji: '件',
		name: 'globex-portal',
		repo: 'globex/portal',
		kind: 'client',
		dojo: 'Globex',
		team: null,
		stack: ['React', 'TypeScript'],
		why: "This repo is Globex's, not your employer's — it's bound to the Globex Dōjō. So it resolves as a client engagement: the Client rung switches on, and Client + Company both apply."
	},
	{
		id: 'site',
		kanji: '件',
		name: 'personal-site',
		repo: 'rin/personal-site',
		kind: 'personal',
		dojo: null,
		team: null,
		stack: ['React'],
		why: 'No Dōjō is bound — this is yours alone. It resolves on the free personal ladder: Personal · Project · Stack. Nothing company or client applies.'
	},
	{
		id: 'mono',
		kanji: '件',
		name: 'agency-monorepo',
		repo: 'studio/monorepo',
		kind: 'client',
		dojo: 'Studio',
		team: null,
		stack: ['React', 'TypeScript'],
		clients: ['Globex', 'Initech'],
		why: "One repo, work for two clients (packages/globex-∗, packages/initech-∗). It isn't your employer's, so it resolves as a client engagement — and because it's bound to two clients, both client rungs apply over the shared company base, each keeping its own derived-learning context."
	}
] as const;

// ── rung templates (broad → specific) ────────────────────────────────────────
// The per-rung rules. Rungs are included per project by lifecycle; the company
// bar sets the locked baseline, the client rung refines it for an engagement, and
// the project/stack rungs try to refine (and, for coverage, RELAX) it. Topics are
// the conflict axes the resolver reasons over.

function companyRung(): Rung {
	return {
		id: 'company',
		scope: 'company',
		kanji: '社',
		name: 'Company',
		label: 'Acme Corp',
		caption: 'your employer · company-wide',
		free: false,
		checkers: [],
		rules: [
			{ topic: 'secrets', text: 'No secrets in source — use the vault, never .env in git', hard: true },
			{ topic: 'pii-logs', text: 'Never log tokens or PII, even at debug level', hard: true },
			{ topic: 'pii-crypto', text: 'Encrypt PII at rest and in transit', hard: true, src: 'compliance' },
			{ topic: 'api-compat', text: 'Public APIs stay backward-compatible two minor versions', hard: false },
			{ topic: 'coverage', text: 'Test coverage ≥ 80% on money- or auth-touching paths', hard: true },
			{ topic: 'autonomy', text: 'Gate irreversible, prod-touching steps for review', hard: true }
		]
	};
}

function clientRung(scopeLabel: string): Rung {
	return {
		id: 'client',
		scope: 'client',
		kanji: '客',
		name: 'Client',
		label: scopeLabel,
		caption: 'engagement · not your employer',
		free: false,
		checkers: [],
		rules: [
			{
				topic: 'isolation',
				text: "This engagement's derived learnings stay in its own context — never merged into the company pool",
				hard: true
			},
			{ topic: 'dpa', text: 'Signed DPA governs any PII in the codebase', hard: true, src: 'compliance' },
			{ topic: 'approvals', text: 'Two maintainer approvals before a lesson publishes to the Dōjō', hard: false }
		]
	};
}

function personalRung(): Rung {
	return {
		id: 'personal',
		scope: 'personal',
		kanji: '己',
		name: 'Personal',
		label: 'You',
		caption: 'your own preferences',
		free: true,
		checkers: [],
		rules: [
			{ topic: 'autonomy', text: 'Prefer run-free autonomy on scratch work', hard: false, relaxOf: true },
			{ topic: 'commits', text: 'Conventional-commit messages', hard: false }
		]
	};
}

function projectRung(proj: PreviewProject): Rung {
	const base = {
		id: 'project',
		scope: 'project' as const,
		kanji: '件',
		name: 'Project',
		label: proj.name,
		caption: 'this repo',
		checkers: [] as const
	};
	if (proj.kind === 'personal') {
		return {
			...base,
			free: true,
			rules: [{ topic: 'export', text: 'Static export — no server-side code', hard: false }]
		};
	}
	return {
		...base,
		free: false,
		rules: [
			{ topic: 'keys', text: 'Rotate signing keys through the token-family table', hard: false },
			{ topic: 'device-flow', text: 'Device-flow codes expire at 10m here, not the default 15m', hard: false },
			{ topic: 'coverage', text: 'Relax coverage to ≥ 60% for this repo', hard: false, relaxOf: true }
		]
	};
}

function stackRung(proj: PreviewProject): Rung {
	return {
		id: 'stack',
		scope: 'stack',
		kanji: '技',
		name: 'Stack',
		label: proj.stack.join(' · '),
		caption: 'language & framework reviewers',
		free: true,
		checkers: ['eslint', 'prettier', 'qlty'],
		rules: [
			{ topic: 'ts-strict', text: 'strict: true — no implicit any (eslint)', hard: true },
			{ topic: 'format', text: 'Format on save (prettier) — no style diffs in review', hard: false },
			{ topic: 'complexity', text: 'Cyclomatic complexity ≤ 10 per function (qlty)', hard: false }
		]
	};
}

/**
 * Build the ladder for a project — the ordered rungs (broad → specific) that
 * apply, per its lifecycle. A `company` project resolves without a Client rung;
 * a `client` project switches the Client rung on (Client + Company both apply);
 * an agency monorepo with multiple clients gets one Client rung per client, each
 * keeping its own isolation context; a `personal` project resolves on the free
 * personal ladder alone. Pure — same input, same rungs.
 */
export function buildLadder(proj: PreviewProject): Rung[] {
	const rungs: Rung[] = [];
	if (proj.kind !== 'personal') rungs.push(companyRung());
	if (proj.clients && proj.clients.length > 0) {
		for (const client of proj.clients) {
			const rung = clientRung(client);
			rungs.push({
				...rung,
				id: `client-${client}`,
				rules: rung.rules.map((r) =>
					r.topic === 'isolation'
						? {
								...r,
								text: `${client}'s derived learnings stay in its own context — never merged with the other client's or the company pool`
							}
						: r
				)
			});
		}
	} else if (proj.kind === 'client') {
		rungs.push(clientRung(proj.dojo ?? 'Client'));
	}
	rungs.push(personalRung(), projectRung(proj), stackRung(proj));
	return rungs;
}

/** Look a sample project up by id (falls back to the first). */
export function projectById(id: string): PreviewProject {
	return PV_PROJECTS.find((p) => p.id === id) ?? PV_PROJECTS[0];
}
