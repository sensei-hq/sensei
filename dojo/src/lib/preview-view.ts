// Pure resolution engine for the effective-constitution preview (Chunk 3). Given
// a project's rungs + rules (built by preview-data `buildLadder`), it composes
// the effective constitution — the rules that actually govern the project once
// the ladder is resolved — by applying two rules, stated plainly in the mockup
// (dojo-preview.jsx comment ~20-23):
//
//   1. a non-negotiable (★/hard) rule LOCKS — no narrower scope can relax it;
//   2. otherwise the more specific scope refines the broader one
//      (Stack > Project > Personal > Client > Company).
//
// Everything here is side-effect-free over plain data (Rung[] in → resolved view
// out), so it unit-tests without a DOM and the `$state` store (preview-state)
// and the component stay thin. The two axes are:
//   · per-rule status — non-negotiable / negotiable / overridden↑ (with the
//     winner noted) — for the LEFT ladder;
//   · per-topic conflicts — topic · winner · what it beat · why — for the RIGHT
//     "conflicts, resolved" cards.

import { scopeRank, type PreviewRule, type Rung, type ScopeId } from './preview-data';

/** A rule's resolved standing on the ladder. `non-negotiable` = a hard lock (★);
 *  `overridden` = it lost its topic (relaxed a lock, or was refined by a more
 *  specific rule); `negotiable` = it applies but can be refined further. */
export type RuleStatus = 'non-negotiable' | 'negotiable' | 'overridden';

/** A rule after resolution — the source rule plus its resolved status and, when
 *  overridden, the scope that beat it (for the "overridden ↑ by Company" note). */
export interface ResolvedRule extends PreviewRule {
	status: RuleStatus;
	/** the winning scope when this rule is `overridden`, else undefined. */
	overriddenBy?: ScopeId;
}

/** A rung after resolution — same shape as the source rung, rules resolved. */
export interface ResolvedRung extends Omit<Rung, 'rules'> {
	rules: ResolvedRule[];
}

/** One settled conflict — two-plus rungs collided on a topic; this records who
 *  won, what they beat, and why (the RIGHT "conflicts, resolved" card). */
export interface Conflict {
	/** the conflict axis. */
	topic: string;
	/** the winning rule's copy. */
	winner: string;
	/** the winning scope. */
	winnerScope: ScopeId;
	/** whether the win is a lock (a hard bar a narrower scope tried to relax). */
	locked: boolean;
	/** the beaten rule's copy (what it beat). */
	lost: string;
	/** the plain-language why. */
	why: string;
}

/** The fully composed effective constitution — the ordered ladder with per-rule
 *  status, the settled conflicts, and the headline counts the summary card shows. */
export interface EffectiveConstitution {
	ladder: ResolvedRung[];
	conflicts: Conflict[];
	/** how many rules actually govern the project (winners, not overridden). */
	totalRules: number;
	/** how many of those are non-negotiable (★). */
	lockedCount: number;
	/** how many scopes composed (rung count). */
	scopeCount: number;
}

// ── per-topic winner resolution (the core precedence rule) ───────────────────

/** A rule with the rung it came from — the unit the resolver compares. */
interface Placed {
	scope: ScopeId;
	rung: Rung;
	rule: PreviewRule;
}

/** Every (rung, rule) pair on a topic, across the whole ladder. */
function placementsForTopic(ladder: readonly Rung[], topic: string): Placed[] {
	const out: Placed[] = [];
	for (const rung of ladder) {
		for (const rule of rung.rules) {
			if (rule.topic === topic) out.push({ scope: rung.scope, rung, rule });
		}
	}
	return out;
}

/**
 * The winning scope RANK for a topic, applying the two rules:
 *   1. if ANY rule on the topic is hard, the BROADEST hard rule locks — a lock
 *      can't be relaxed by a narrower scope (nor by a narrower hard rule, which
 *      would only tighten; the broad lock is what "governs"). We surface the
 *      broadest hard rank so the story reads "Company marks it non-negotiable".
 *   2. otherwise (all soft) the MOST SPECIFIC rule wins — it refines the broader.
 * `placements` is assumed non-empty (callers only pass topics with rules).
 *
 * The winner is a RANK, not a single placement, so parallel rules at the same
 * scope (e.g. two client rungs both locking `isolation`, each for its own client)
 * are co-winners rather than one silently overriding the other.
 */
function winningRank(placements: readonly Placed[]): number {
	const hard = placements.filter((p) => p.rule.hard);
	const pool = hard.length > 0 ? hard : placements;
	// hard → broadest (lowest rank) locks; soft → most specific (highest rank) wins.
	const pick = hard.length > 0 ? Math.min : Math.max;
	return pick(...pool.map((p) => scopeRank(p.scope)));
}

/** Whether a placement wins (or co-wins) its topic — it sits at the winning rank
 *  and, when the topic is locked (any hard rule present), is itself hard. This
 *  lets two same-rank hard rules co-win while a soft rule at the winning rank of a
 *  locked topic (there won't be one for the same scope, but be defensive) doesn't. */
function isWinningPlacement(placements: readonly Placed[], target: Placed): boolean {
	const rank = winningRank(placements);
	const locked = placements.some((p) => p.rule.hard);
	if (scopeRank(target.scope) !== rank) return false;
	return locked ? target.rule.hard : true;
}

/** The representative winning placement (broadest at the winning rank) — used to
 *  name the winning scope in a conflict card. */
function representativeWinner(placements: readonly Placed[]): Placed {
	const rank = winningRank(placements);
	const atRank = placements.filter((p) => scopeRank(p.scope) === rank);
	const locked = placements.some((p) => p.rule.hard);
	const pool = locked ? atRank.filter((p) => p.rule.hard) : atRank;
	return pool[0] ?? placements[0];
}

/** Is a topic contested — more than one distinct rule across the ladder? A single
 *  rule (even if it appears once) is not a conflict; two rules on a topic are. */
function isContested(placements: readonly Placed[]): boolean {
	return placements.length > 1;
}

// ── per-rule status (the LEFT ladder tags) ───────────────────────────────────

/**
 * The resolved status of one rule, given the whole ladder. A rule is
 * `non-negotiable` when it is the hard winner of its topic; `overridden` when a
 * different rule won its topic (it relaxed a lock, or was refined by a more
 * specific rule); `negotiable` when it applies (won a soft topic, or stands
 * uncontested and is not hard).
 */
export function ruleStatus(ladder: readonly Rung[], rung: Rung, rule: PreviewRule): RuleStatus {
	const placements = placementsForTopic(ladder, rule.topic);
	const target = placements.find((p) => p.scope === rung.scope && p.rule === rule) ?? {
		scope: rung.scope,
		rung,
		rule
	};
	if (isWinningPlacement(placements, target)) return rule.hard ? 'non-negotiable' : 'negotiable';
	return 'overridden';
}

/** The scope that beat a rule (only meaningful for an overridden rule). */
export function overriddenBy(ladder: readonly Rung[], rule: PreviewRule): ScopeId | undefined {
	const placements = placementsForTopic(ladder, rule.topic);
	if (!isContested(placements)) return undefined;
	return representativeWinner(placements).scope;
}

// ── conflicts (the RIGHT cards) ──────────────────────────────────────────────

/** A readable label for a scope (for the "why" narration). */
function scopeName(scope: ScopeId): string {
	switch (scope) {
		case 'company':
			return 'Company';
		case 'client':
			return 'Client';
		case 'personal':
			return 'Personal';
		case 'project':
			return 'Project';
		case 'stack':
			return 'Stack';
	}
}

/**
 * The settled conflicts across the ladder — one per contested topic (a topic
 * with more than one rule). Each records the winner, the highest-ranked beaten
 * rule (what it most concretely beat), whether the win is a lock, and a plain
 * "why". Uncontested topics produce no card (passthrough).
 */
export function conflictsFor(ladder: readonly Rung[]): Conflict[] {
	const seen = new Set<string>();
	const conflicts: Conflict[] = [];
	for (const rung of ladder) {
		for (const rule of rung.rules) {
			if (seen.has(rule.topic)) continue;
			seen.add(rule.topic);
			const placements = placementsForTopic(ladder, rule.topic);
			if (!isContested(placements)) continue;
			// A real conflict needs a real loser. Parallel co-winners (e.g. two client
			// rungs each locking their own isolation) collide on the topic key but
			// nobody is overridden → no conflict card.
			const losers = placements.filter((p) => !isWinningPlacement(placements, p));
			if (losers.length === 0) continue;
			const winner = representativeWinner(placements);
			const locked = winner.rule.hard;
			// The beaten rule we narrate: the most-specific loser (the one that most
			// concretely tried to change the winner) reads best on the card.
			const beaten = losers.reduce((a, b) => (scopeRank(b.scope) > scopeRank(a.scope) ? b : a));
			conflicts.push({
				topic: rule.topic,
				winner: winner.rule.text,
				winnerScope: winner.scope,
				locked,
				lost: beaten.rule.text,
				why: locked
					? `${scopeName(winner.scope)} marks this non-negotiable (★), so the narrower ${scopeName(beaten.scope)} scope can't relax it.`
					: `The more specific ${scopeName(winner.scope)} scope refines the broader ${scopeName(beaten.scope)} one.`
			});
		}
	}
	return conflicts;
}

// ── the whole composed view ──────────────────────────────────────────────────

/** Resolve every rung's rules to their status + winner, in ladder order. */
function resolveLadder(ladder: readonly Rung[]): ResolvedRung[] {
	return ladder.map((rung) => ({
		...rung,
		rules: rung.rules.map((rule) => {
			const status = ruleStatus(ladder, rung, rule);
			return {
				...rule,
				status,
				overriddenBy: status === 'overridden' ? overriddenBy(ladder, rule) : undefined
			};
		})
	}));
}

/** Count the effective (winning, non-overridden) rules across resolved rungs. */
export function totalRuleCount(ladder: readonly ResolvedRung[]): number {
	return ladder.reduce((n, r) => n + r.rules.filter((x) => x.status !== 'overridden').length, 0);
}

/** Count the non-negotiable (★) winners across resolved rungs. */
export function lockedCount(ladder: readonly ResolvedRung[]): number {
	return ladder.reduce((n, r) => n + r.rules.filter((x) => x.status === 'non-negotiable').length, 0);
}

/**
 * The full effective constitution for a ladder — the composed view the screen
 * renders: the ordered ladder (per-rule status), the settled conflicts, and the
 * headline counts. Pure: same ladder in, same view out.
 */
export function resolveConstitution(ladder: readonly Rung[]): EffectiveConstitution {
	const resolved = resolveLadder(ladder);
	return {
		ladder: resolved,
		conflicts: conflictsFor(ladder),
		totalRules: totalRuleCount(resolved),
		lockedCount: lockedCount(resolved),
		scopeCount: ladder.length
	};
}
