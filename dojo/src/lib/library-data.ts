// Constitution-library catalog — the predefined rule/template set a Dōjō starts
// from (Chunk 2 of the Dōjō IA rebuild; mockup dojo-library.jsx `LIB_CATS`,
// `LIB_PACKS`, `LIB_LEVELS`). Prevention over cure: pull proven principles,
// patterns, compliance controls, stack reviewers and design guardrails INTO the
// constitution rather than authoring every rule by hand.
//
// This module is DATA only — pure typed constants, no DOM, no fetch — so the
// screen is driven from one source of truth and the selection math in
// `library-view.ts` can be unit-tested against it. The taxonomy (the six areas)
// stays consistent with the seeded constitution in
// database/ddl/procedure/dojo/seed_default_governance.ddl; the catalog itself
// mirrors the mockup's `LIB_PACKS`. `/v1` wiring to real `dojo.shared_rules` is
// deferred (see the plan's Chunk 2), so the packs are local for now.

/** A level a rule can be pulled in at — cascades down the same ladder as
 *  authored governance (mockup `LIB_LEVELS`). */
export type LibLevelId = 'org' | 'team' | 'proj' | 'stack';

/** A selectable level, with its label + kanji glyph (for the LevelPills). */
export interface LibLevel {
	id: LibLevelId;
	label: string;
	kanji: string;
}

/** The four levels, in ladder order (Org → Team → Project → Stack). */
export const LIB_LEVELS: readonly LibLevel[] = [
	{ id: 'org', label: 'Org', kanji: '社' },
	{ id: 'team', label: 'Team', kanji: '組' },
	{ id: 'proj', label: 'Project', kanji: '件' },
	{ id: 'stack', label: 'Stack', kanji: '技' }
] as const;

/** An area (left-rail category): the six governance areas (mockup `LIB_CATS`). */
export type LibAreaId = 'core' | 'arch' | 'sec' | 'comp' | 'stack' | 'design';

/** An area with its kanji, display name and one-line subtitle. */
export interface LibArea {
	id: LibAreaId;
	kanji: string;
	name: string;
	sub: string;
}

/** The six areas, in rail order. Kept in step with the seeded constitution's
 *  rule_type families (Constitution/Quality/Architecture/… → these areas). */
export const LIB_CATS: readonly LibArea[] = [
	{ id: 'core', kanji: '理', name: 'Core principles', sub: 'YAGNI · DRY · SRP · testability' },
	{ id: 'arch', kanji: '紋', name: 'Architecture & patterns', sub: 'GoF · patterns.dev · modularity' },
	{ id: 'sec', kanji: '盾', name: 'Security', sub: 'OWASP · secrets · authz' },
	{ id: 'comp', kanji: '法', name: 'Compliance', sub: 'HIPAA · PCI · SOC 2 · GDPR' },
	{ id: 'stack', kanji: '技', name: 'Language & stack', sub: 'reviewers · lint · format · complexity' },
	{ id: 'design', kanji: '意', name: 'Design system & UI', sub: 'tokens · atomic · no stray pixels' }
] as const;

/** One rule inside a pack. `id` is a stable key (pack id + index) used for
 *  selection state; `text` is the rule copy; `hard` marks a hard guard —
 *  auto-locked non-negotiable once included (can't be relaxed downstream). */
export interface LibRule {
	id: string;
	text: string;
	hard: boolean;
}

/** A pack: a named, sourced bundle of cherry-pickable rules within one area.
 *  `defLevel` is the level it naturally applies at; `rec` marks the ones we
 *  recommend for a fresh Dōjō; `regulated` marks a compliance framework;
 *  `checkers` are the wired open-source tools (stack reviewers). */
export interface LibPack {
	id: string;
	area: LibAreaId;
	name: string;
	source: string;
	defLevel: LibLevelId;
	rec: boolean;
	regulated: boolean;
	checkers: readonly string[];
	rules: readonly LibRule[];
}

// ── raw pack authoring shape ─────────────────────────────────────────────────
// The authored packs, grouped by area — a terse literal that mirrors the
// mockup's `LIB_PACKS` exactly. Rule ids are derived below so authoring stays
// one-line-per-rule (the mockup keyed selection on rule text; we key on a
// stable id so two packs can share identical rule text — e.g. "Single
// Responsibility" appears in both First principles and SOLID — without
// collapsing into one toggle).
interface RawRule {
	t: string;
	hard?: boolean;
}
interface RawPack {
	id: string;
	name: string;
	source: string;
	defLevel: LibLevelId;
	rec?: boolean;
	regulated?: boolean;
	checkers?: readonly string[];
	rules: readonly RawRule[];
}

const RAW_PACKS: Record<LibAreaId, readonly RawPack[]> = {
	core: [
		{
			id: 'core-first',
			name: 'First principles',
			source: 'sensei core',
			defLevel: 'org',
			rec: true,
			rules: [
				{ t: 'YAGNI — build only what a current requirement needs' },
				{ t: 'DRY — a single source of truth for every piece of knowledge' },
				{ t: 'KISS — prefer the simplest design that works' },
				{ t: 'Single Responsibility — one reason to change per unit' },
				{ t: 'Composition over inheritance' }
			]
		},
		{
			id: 'core-solid',
			name: 'SOLID',
			source: 'R. Martin',
			defLevel: 'org',
			rec: true,
			rules: [
				{ t: 'Single Responsibility Principle' },
				{ t: 'Open/Closed — open to extension, closed to modification' },
				{ t: 'Liskov Substitution — subtypes honour their base contract' },
				{ t: 'Interface Segregation — no client depends on unused methods' },
				{ t: 'Dependency Inversion — depend on abstractions, not concretions' }
			]
		},
		{
			id: 'core-test',
			name: 'Testability & quality',
			source: 'sensei core',
			defLevel: 'org',
			rules: [
				{ t: 'Every behaviour change ships with a test that would fail without it' },
				{ t: 'Pure functions for logic; push side-effects to the edges' },
				{ t: 'No untested error path on a money- or auth-touching flow', hard: true },
				{ t: 'Fast unit suite (<10s) gates every commit' }
			]
		}
	],
	arch: [
		{
			id: 'arch-modular',
			name: 'Modularity & boundaries',
			source: 'sensei core',
			defLevel: 'org',
			rec: true,
			rules: [
				{ t: 'Feature-sliced modules — cohesive inside, thin contracts between' },
				{ t: 'Dependencies point inward; domain has no framework imports' },
				{ t: 'Wrap third-party SDKs behind an owned interface for testability' },
				{ t: 'No circular dependencies between modules', hard: true }
			]
		},
		{
			id: 'arch-gof',
			name: 'Design patterns · OO (GoF)',
			source: 'Gang of Four',
			defLevel: 'team',
			rules: [
				{ t: 'Prefer Strategy over branching on type' },
				{ t: 'Factory / Builder for multi-step or conditional construction' },
				{ t: 'Adapter at every third-party boundary' },
				{ t: 'Observer / pub-sub for cross-module events, not direct calls' }
			]
		},
		{
			id: 'arch-patternsdev',
			name: 'Web patterns',
			source: 'patterns.dev',
			defLevel: 'stack',
			rules: [
				{ t: 'Container/Presentational split for view logic' },
				{ t: 'Code-split at the route; lazy-load below the fold' },
				{ t: 'Debounce / throttle expensive event handlers' },
				{ t: 'Memoize derived data, not whole trees' }
			]
		}
	],
	sec: [
		{
			id: 'sec-owasp',
			name: 'OWASP baseline',
			source: 'OWASP ASVS',
			defLevel: 'org',
			rec: true,
			rules: [
				{ t: 'No secrets in source — use the vault, never .env in git', hard: true },
				{ t: 'Never log tokens or PII, even at debug level', hard: true },
				{ t: 'Parameterised queries only — no string-built SQL', hard: true },
				{ t: 'Validate & encode every external input at the boundary' },
				{ t: 'Authorize on every request, server-side, deny by default' }
			]
		},
		{
			id: 'sec-supply',
			name: 'Supply chain',
			source: 'SLSA',
			defLevel: 'org',
			rules: [
				{ t: 'Pin dependency versions; no floating majors' },
				{ t: 'Block a merge on a high/critical advisory', hard: true },
				{ t: 'Generate an SBOM on every release build' }
			]
		}
	],
	comp: [
		{
			id: 'comp-hipaa',
			name: 'HIPAA',
			source: '45 CFR §164',
			defLevel: 'org',
			regulated: true,
			rules: [
				{ t: 'Encrypt PHI at rest and in transit', hard: true },
				{ t: 'Immutable audit log of every PHI access', hard: true },
				{ t: 'Minimum-necessary access — least privilege on PHI' },
				{ t: 'Signed BAA before any PHI leaves the boundary', hard: true }
			]
		},
		{
			id: 'comp-pci',
			name: 'PCI DSS',
			source: 'PCI SSC v4',
			defLevel: 'org',
			regulated: true,
			rules: [
				{ t: 'Never store CVV; tokenize the PAN', hard: true },
				{ t: 'Cardholder data isolated to a segmented scope' },
				{ t: 'Quarterly access review of the CDE' }
			]
		},
		{
			id: 'comp-soc2',
			name: 'SOC 2',
			source: 'AICPA TSC',
			defLevel: 'org',
			regulated: true,
			rules: [
				{ t: 'Change management — every prod change is reviewed & traceable', hard: true },
				{ t: 'Access granted by role, revoked on offboard within 24h' },
				{ t: 'Encrypted backups, restore tested quarterly' }
			]
		},
		{
			id: 'comp-gdpr',
			name: 'GDPR',
			source: 'EU 2016/679',
			defLevel: 'org',
			regulated: true,
			rules: [
				{ t: 'Lawful basis recorded for every category of personal data', hard: true },
				{ t: 'Honour erasure & export requests within 30 days' },
				{ t: 'Data-minimisation — collect only what the purpose needs' },
				{ t: 'Privacy by design on new data flows' }
			]
		}
	],
	stack: [
		{
			id: 'stk-ts',
			name: 'TypeScript / JavaScript',
			source: 'reviewer',
			defLevel: 'stack',
			rec: true,
			checkers: ['eslint', 'prettier', 'qlty'],
			rules: [
				{ t: 'strict: true — no implicit any', hard: true },
				{ t: 'No unused exports, vars or imports (eslint)' },
				{ t: 'Format on save (prettier) — no style diffs in review' },
				{ t: 'Cyclomatic complexity ≤ 10 per function (qlty)' }
			]
		},
		{
			id: 'stk-py',
			name: 'Python',
			source: 'reviewer',
			defLevel: 'stack',
			checkers: ['ruff', 'black', 'mypy', 'qlty'],
			rules: [
				{ t: 'ruff lint clean — pyflakes + isort + pyupgrade' },
				{ t: 'black formatting enforced in CI' },
				{ t: 'Type hints on public functions; mypy strict on core' },
				{ t: 'No function over 50 lines / complexity 10 (qlty)' }
			]
		},
		{
			id: 'stk-rust',
			name: 'Rust',
			source: 'reviewer',
			defLevel: 'stack',
			checkers: ['clippy', 'rustfmt'],
			rules: [
				{ t: 'clippy clean at deny(warnings)', hard: true },
				{ t: 'rustfmt enforced; no manual formatting' },
				{ t: 'No unwrap() on a fallible path outside tests' }
			]
		},
		{
			id: 'stk-go',
			name: 'Go',
			source: 'reviewer',
			defLevel: 'stack',
			checkers: ['golangci-lint', 'gofmt', 'qlty'],
			rules: [
				{ t: 'golangci-lint clean before merge' },
				{ t: 'gofmt / goimports enforced' },
				{ t: 'Errors wrapped with context, never discarded' }
			]
		}
	],
	design: [
		{
			id: 'dsn-tokens',
			name: 'Semantic tokens',
			source: 'design system',
			defLevel: 'org',
			rec: true,
			rules: [
				{ t: 'Color from semantic tokens only — no raw #hex / rgba / oklch', hard: true },
				{ t: 'Type from the fixed scale — never a literal font-size', hard: true },
				{ t: 'Spacing from the 4px grid — no stray 18px / 5px values' },
				{ t: 'Never redefine tokens locally — link the canonical stylesheet', hard: true }
			]
		},
		{
			id: 'dsn-atomic',
			name: 'Atomic components',
			source: 'design system',
			defLevel: 'team',
			rules: [
				{ t: 'Reuse the shared component; never hand-roll a duplicate' },
				{ t: 'One missing variant is added to the component, not forked inline' },
				{ t: 'Geometry-only inline styles; color/space/type via classes' },
				{ t: 'Compose screens from primitives, not bespoke markup' }
			]
		},
		{
			id: 'dsn-a11y',
			name: 'Accessibility',
			source: 'WCAG 2.2 AA',
			defLevel: 'org',
			rules: [
				{ t: '4.5:1 text contrast; 3:1 for large text & UI', hard: true },
				{ t: 'Every interactive target ≥ 44px and keyboard-reachable' },
				{ t: 'Semantic landmarks & labels on every control' }
			]
		}
	]
};

// ── normalize raw packs → typed catalog ──────────────────────────────────────
// Assign each rule a stable id (`{packId}:{index}`) and fill the optional flags
// so downstream code never branches on `undefined`.
function normalizePack(raw: RawPack, area: LibAreaId): LibPack {
	return {
		id: raw.id,
		area,
		name: raw.name,
		source: raw.source,
		defLevel: raw.defLevel,
		rec: raw.rec ?? false,
		regulated: raw.regulated ?? false,
		checkers: raw.checkers ?? [],
		rules: raw.rules.map((r, i) => ({ id: `${raw.id}:${i}`, text: r.t, hard: r.hard ?? false }))
	};
}

/** The full pack catalog, grouped by area (the LIB_PACKS equivalent). */
export const LIB_PACKS: Record<LibAreaId, readonly LibPack[]> = {
	core: RAW_PACKS.core.map((p) => normalizePack(p, 'core')),
	arch: RAW_PACKS.arch.map((p) => normalizePack(p, 'arch')),
	sec: RAW_PACKS.sec.map((p) => normalizePack(p, 'sec')),
	comp: RAW_PACKS.comp.map((p) => normalizePack(p, 'comp')),
	stack: RAW_PACKS.stack.map((p) => normalizePack(p, 'stack')),
	design: RAW_PACKS.design.map((p) => normalizePack(p, 'design'))
};

/** Every pack across every area, flattened (for catalog-wide lookups). */
export const ALL_PACKS: readonly LibPack[] = (Object.keys(LIB_PACKS) as LibAreaId[]).flatMap(
	(area) => LIB_PACKS[area]
);

/** The packs in one area (empty for an unknown area). Convenience for the
 *  component so it reads the catalog by area without importing view helpers. */
export function packsForArea(area: LibAreaId): readonly LibPack[] {
	return LIB_PACKS[area] ?? [];
}
