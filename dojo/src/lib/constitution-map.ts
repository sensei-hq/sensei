// Pure wire→kit mapping for the constitution surfaces (W1). Turns the rows the
// constitution read route returns (dojo.shared_rules ⋈ sensei.namespaces) into the
// kit shapes the screens already take: the personal LADDER (`KitLadderRung[]`, for
// ScrConstitution) and the org authoring SECTIONS (`KitConstitutionSection[]`, for
// ScrOrgLadder). Side-effect-free so the mapping unit-tests without a DOM or a Worker.
//
// Scope vocabulary + precedence come from `sensei.scopes` (key · level 0→70, higher =
// more specific): general·user·organization·client·technology·team·project·repository.
// Ladder glyphs match the kit (社 Company · 客 Client · 己 Personal · 件 Project · 技
// Stack) and section groups (社 Company · 組 Teams · 技 Stacks). See
// docs/blueprints/2026-07-27-dojo-w1-wiring-slice-pattern.md.

import type {
	KitLadderRung,
	KitRule,
	KitConstitutionSection,
	KitConflict
} from './components/kit/types';

/** A rule row as the constitution read route returns it — the display subset of the
 *  `shared_rules ⋈ namespaces` join. */
export interface ConstitutionRule {
	scope_key: string;
	namespace_name: string;
	title: string;
	/** advisory · recommended · required · mandatory (sensei.enforcement). */
	enforcement: string;
}

interface ScopeMeta {
	label: string;
	kanji: string;
	/** The org authoring group (KitConstitutionSection.group): Company · Teams · Stacks. */
	group: string;
	/** Precedence rank from sensei.scopes.level — lower = broader; the ladder orders by it. */
	level: number;
}

/** scope_key → kit meta (mirrors `sensei.scopes`). Unknown keys fall back to a neutral
 *  most-specific rung so a new scope never breaks the ladder (name still comes from data). */
const SCOPE_META: Record<string, ScopeMeta> = {
	general: { label: 'General', kanji: '全', group: 'Company', level: 0 },
	user: { label: 'Personal', kanji: '己', group: 'Company', level: 10 },
	organization: { label: 'Company', kanji: '社', group: 'Company', level: 20 },
	client: { label: 'Client', kanji: '客', group: 'Company', level: 30 },
	technology: { label: 'Stack', kanji: '技', group: 'Stacks', level: 40 },
	team: { label: 'Team', kanji: '組', group: 'Teams', level: 50 },
	project: { label: 'Project', kanji: '件', group: 'Stacks', level: 60 },
	repository: { label: 'Repository', kanji: '庫', group: 'Stacks', level: 70 }
};

const FALLBACK: ScopeMeta = { label: 'Other', kanji: '守', group: 'Stacks', level: 999 };

function meta(scopeKey: string): ScopeMeta {
	return SCOPE_META[scopeKey] ?? FALLBACK;
}

/** A rule is a hard ★ lock when its enforcement is `mandatory` — no narrower scope can
 *  relax it (mirrors sensei.enforcement's top tier). */
export function isHardRule(enforcement: string): boolean {
	return enforcement === 'mandatory';
}

function toKitRule(r: ConstitutionRule): KitRule {
	return { kanji: '守', text: r.title, hard: isHardRule(r.enforcement), level: meta(r.scope_key).label };
}

function caption(n: number): string {
	return `${n} ${n === 1 ? 'rule' : 'rules'}`;
}

/** Group rules by scope_key (insertion order preserved), then order the scopes
 *  broad→specific by their `sensei.scopes.level`. */
function scopesByLevel(rules: ConstitutionRule[]): [string, ConstitutionRule[]][] {
	const groups = new Map<string, ConstitutionRule[]>();
	for (const r of rules) {
		const g = groups.get(r.scope_key);
		if (g) g.push(r);
		else groups.set(r.scope_key, [r]);
	}
	return [...groups.entries()].sort((a, b) => meta(a[0]).level - meta(b[0]).level);
}

/**
 * The personal constitution LADDER (`KitLadderRung[]`) — one rung per scope present,
 * ordered broad→specific, each carrying its rules with the mandatory ★ lock surfaced.
 * Empty in → empty out (honest empty; never a fabricated or another tenant's rung).
 */
export function rulesToLadder(rules: ConstitutionRule[]): KitLadderRung[] {
	return scopesByLevel(rules).map(([scopeKey, rs]) => {
		const m = meta(scopeKey);
		return {
			id: scopeKey,
			kanji: m.kanji,
			scope: m.label,
			name: rs[0]?.namespace_name ?? m.label,
			caption: caption(rs.length),
			rules: rs.map(toKitRule)
		};
	});
}

// ── Per-project resolved constitution (F4) ────────────────────────────────────
// The daemon federates the folder's RESOLVED constitution (authority resolution,
// dedup + mandatory locks + the discards the ladder made — all server-side) as
// `dojo.projects.constitution`. Here we map that wire shape onto the SAME kit
// ladder + conflicts the shipped `ScrProjectPreview` renders, reusing
// `rulesToLadder` for the rungs (the dōjō only DISPLAYS; it never re-resolves).

/** The `RelayConstitution` wire shape the daemon federates (mirrors
 *  `dojo_protocol::relay::RelayConstitution`). Rules carry the daemon's own
 *  `scope_key`/`enforcement`; conflicts are the discards, winner/loser decided
 *  server-side. */
export interface RelayConstitution {
	rules: { scope_key: string; namespace?: string | null; title: string; enforcement: string }[];
	conflicts: {
		topic: string;
		loser_scope: string;
		winner_scope: string;
		why: string;
		locked: boolean;
	}[];
	locks: number;
}

/** Federated constitution rules → the `ConstitutionRule[]` `rulesToLadder` takes,
 *  so the per-project ladder reuses the shared scope→rung mapping. A rule with no
 *  namespace falls back to its scope label (honest, never a blank rung name). */
export function relayRules(c: RelayConstitution): ConstitutionRule[] {
	return c.rules.map((r) => ({
		scope_key: r.scope_key,
		namespace_name: r.namespace ?? meta(r.scope_key).label,
		title: r.title,
		enforcement: r.enforcement
	}));
}

/** The discards the ladder made → `KitConflict[]` for the "Discarded by the
 *  ladder" section. The winner/loser scopes are the daemon's decision (the dōjō
 *  only labels them); both sides share the discarded rule's `topic` text. */
export function relayToKitConflicts(c: RelayConstitution): KitConflict[] {
	return c.conflicts.map((x, i) => ({
		id: `conflict-${i}`,
		topic: x.topic,
		loser: { level: meta(x.loser_scope).label, text: x.topic },
		winner: { level: meta(x.winner_scope).label, text: x.topic },
		why: x.why,
		locked: x.locked
	}));
}

/**
 * The org authoring constitution as SECTIONS (`KitConstitutionSection[]`) grouped
 * Company/Teams/Stacks — the maintainer surface. One section per scope present, ordered
 * broad→specific. Empty in → empty out.
 */
export function rulesToSections(rules: ConstitutionRule[]): KitConstitutionSection[] {
	return scopesByLevel(rules).map(([scopeKey, rs]) => {
		const m = meta(scopeKey);
		return {
			id: scopeKey,
			kanji: m.kanji,
			scope: rs[0]?.namespace_name ?? m.label,
			group: m.group,
			caption: caption(rs.length),
			rules: rs.map(toKitRule)
		};
	});
}
