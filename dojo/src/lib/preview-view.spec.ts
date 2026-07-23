import { describe, expect, it } from 'vitest';
import {
	buildLadder,
	projectById,
	scopeRank,
	type PreviewRule,
	type Rung
} from '$lib/preview-data';
import {
	conflictsFor,
	lockedCount,
	resolveConstitution,
	ruleStatus,
	totalRuleCount,
	type ResolvedRung,
	type RuleStatus
} from '$lib/preview-view';

// Pure resolution-engine tests for the effective-constitution preview. This is
// the heart of Chunk 3: given a project's rungs + rules, compute the effective
// ruleset applying two rules, stated plainly in the mockup (dojo-preview.jsx
// comment ~20-23):
//   1. a non-negotiable (★/hard) rule LOCKS — no narrower scope can relax it;
//   2. otherwise the more specific scope refines the broader one.
// The tests below exhaustively cover: a mandatory rung-rule that a more-specific
// rung tries to relax → mandatory wins (locked); a non-mandatory conflict →
// most-specific wins; a no-conflict passthrough; and the company↔client
// reclassification changing which rungs apply.

// ── small hand-built ladders (isolated from the real catalog) ────────────────

function rung(id: Rung['scope'], rules: PreviewRule[], over: Partial<Rung> = {}): Rung {
	return {
		id,
		scope: id,
		kanji: '·',
		name: id,
		label: id,
		caption: '',
		free: false,
		checkers: [],
		rules,
		...over
	};
}

/** A company rung with a hard-locked coverage bar, and a project rung that tries
 *  to relax it — the canonical mandatory-lock case. */
function lockLadder(): Rung[] {
	return [
		rung('company', [{ topic: 'coverage', text: 'coverage ≥ 80%', hard: true }]),
		rung('project', [{ topic: 'coverage', text: 'relax coverage to ≥ 60%', hard: false, relaxOf: true }])
	];
}

/** A company rung with a SOFT rule and a more-specific project rung that refines
 *  it — the most-specific-wins case (no lock involved). */
function refineLadder(): Rung[] {
	return [
		rung('company', [{ topic: 'timeout', text: 'default timeout 15m', hard: false }]),
		rung('project', [{ topic: 'timeout', text: 'timeout 10m here', hard: false }])
	];
}

describe('scopeRank / ladder order (broad → specific)', () => {
	it('ranks company broadest and stack most specific', () => {
		expect(scopeRank('company')).toBeLessThan(scopeRank('client'));
		expect(scopeRank('client')).toBeLessThan(scopeRank('personal'));
		expect(scopeRank('personal')).toBeLessThan(scopeRank('project'));
		expect(scopeRank('project')).toBeLessThan(scopeRank('stack'));
	});
});

describe('ruleStatus — mandatory-lock beats specificity', () => {
	it('a hard company rule stays non-negotiable', () => {
		const ladder = lockLadder();
		const company = ladder[0];
		expect(ruleStatus(ladder, company, company.rules[0])).toBe<RuleStatus>('non-negotiable');
	});

	it('a more-specific rule that tries to relax a hard bar is overridden (locked)', () => {
		const ladder = lockLadder();
		const project = ladder[1];
		// The project's "relax to ≥ 60%" loses to the company hard bar → overridden↑.
		expect(ruleStatus(ladder, project, project.rules[0])).toBe<RuleStatus>('overridden');
	});
});

describe('ruleStatus — most-specific-wins otherwise', () => {
	it('a soft broader rule loses to a more-specific rule on the same topic', () => {
		const ladder = refineLadder();
		const company = ladder[0];
		const project = ladder[1];
		// No hard rule anywhere on `timeout` → the more specific (project) wins.
		expect(ruleStatus(ladder, project, project.rules[0])).toBe<RuleStatus>('negotiable');
		expect(ruleStatus(ladder, company, company.rules[0])).toBe<RuleStatus>('overridden');
	});

	it('a soft rule with no competitor on its topic passes through as negotiable', () => {
		const ladder = [rung('company', [{ topic: 'solo', text: 'just this', hard: false }])];
		expect(ruleStatus(ladder, ladder[0], ladder[0].rules[0])).toBe<RuleStatus>('negotiable');
	});

	it('a hard rule with no competitor passes through as non-negotiable', () => {
		const ladder = [rung('company', [{ topic: 'solo', text: 'locked', hard: true }])];
		expect(ruleStatus(ladder, ladder[0], ladder[0].rules[0])).toBe<RuleStatus>('non-negotiable');
	});
});

describe('conflictsFor — topic · winner · what it beat · why', () => {
	it('mandatory-lock: company wins over the project relax, marked locked', () => {
		const conflicts = conflictsFor(lockLadder());
		const coverage = conflicts.find((c) => c.topic === 'coverage')!;
		expect(coverage).toBeTruthy();
		expect(coverage.winnerScope).toBe('company');
		expect(coverage.locked).toBe(true);
		expect(coverage.lost).toContain('relax coverage');
		expect(coverage.why.toLowerCase()).toContain('non-negotiable');
	});

	it('most-specific-wins: the project rule wins a soft conflict, not locked', () => {
		const conflicts = conflictsFor(refineLadder());
		const timeout = conflicts.find((c) => c.topic === 'timeout')!;
		expect(timeout).toBeTruthy();
		expect(timeout.winnerScope).toBe('project');
		expect(timeout.locked).toBe(false);
		expect(timeout.lost).toContain('15m');
	});

	it('no conflict → no conflict entry (passthrough)', () => {
		const ladder = [
			rung('company', [{ topic: 'a', text: 'one', hard: false }]),
			rung('project', [{ topic: 'b', text: 'two', hard: false }])
		];
		expect(conflictsFor(ladder)).toEqual([]);
	});

	it('a stricter more-specific rule wins a soft-vs-soft conflict (client over company)', () => {
		const ladder = [
			rung('company', [{ topic: 'approvals', text: '1 approval', hard: false }]),
			rung('client', [{ topic: 'approvals', text: '2 approvals', hard: false }])
		];
		const c = conflictsFor(ladder).find((x) => x.topic === 'approvals')!;
		expect(c.winnerScope).toBe('client');
		expect(c.locked).toBe(false);
	});
});

describe('resolveConstitution — the whole composed view', () => {
	it('counts effective rules (winners) and the locked (non-negotiable) subset', () => {
		const eff = resolveConstitution(lockLadder());
		// coverage topic resolves to ONE effective rule (the company hard bar); the
		// relax rule is overridden and does not count toward the total.
		expect(eff.totalRules).toBe(1);
		expect(eff.lockedCount).toBe(1);
		expect(eff.scopeCount).toBe(2);
	});

	it('marks each ladder rung rule with its resolved status', () => {
		const eff = resolveConstitution(lockLadder());
		const company = eff.ladder.find((r) => r.scope === 'company')!;
		const project = eff.ladder.find((r) => r.scope === 'project')!;
		expect(company.rules[0].status).toBe<RuleStatus>('non-negotiable');
		expect(project.rules[0].status).toBe<RuleStatus>('overridden');
		// the overridden rule notes who beat it
		expect(project.rules[0].overriddenBy).toBe('company');
	});

	it('preserves rung order broad → specific', () => {
		const eff = resolveConstitution(buildLadder(projectById('globex')));
		const ranks = eff.ladder.map((r) => scopeRank(r.scope));
		const sorted = [...ranks].sort((a, b) => a - b);
		expect(ranks).toEqual(sorted);
	});
});

// ── the four sample lifecycles + the reclassification ────────────────────────

describe('company project (lumen-auth) — no Client rung', () => {
	const eff = resolveConstitution(buildLadder(projectById('auth')));

	it('has Company but no Client rung', () => {
		const scopes = eff.ladder.map((r) => r.scope);
		expect(scopes).toContain('company');
		expect(scopes).not.toContain('client');
		expect(scopes).toContain('personal');
		expect(scopes).toContain('project');
		expect(scopes).toContain('stack');
	});

	it('locks coverage against the project relax and gates autonomy over the personal preference', () => {
		const coverage = eff.conflicts.find((c) => c.topic === 'coverage')!;
		expect(coverage.winnerScope).toBe('company');
		expect(coverage.locked).toBe(true);
		const autonomy = eff.conflicts.find((c) => c.topic === 'autonomy')!;
		expect(autonomy.winnerScope).toBe('company');
		expect(autonomy.locked).toBe(true);
		expect(autonomy.lost.toLowerCase()).toContain('run-free');
	});
});

describe('client project (globex-portal) — Client + Company both apply', () => {
	const eff = resolveConstitution(buildLadder(projectById('globex')));

	it('switches the Client rung on', () => {
		const scopes = eff.ladder.map((r) => r.scope);
		expect(scopes).toContain('company');
		expect(scopes).toContain('client');
	});

	it('still locks coverage (company) and adds the client isolation lock', () => {
		expect(eff.conflicts.find((c) => c.topic === 'coverage')?.locked).toBe(true);
		const isolation = eff.ladder
			.find((r) => r.scope === 'client')!
			.rules.find((r) => r.topic === 'isolation')!;
		expect(isolation.status).toBe<RuleStatus>('non-negotiable');
	});
});

describe('personal project (personal-site) — free personal ladder alone', () => {
	const eff = resolveConstitution(buildLadder(projectById('site')));

	it('has no Company and no Client rung', () => {
		const scopes = eff.ladder.map((r) => r.scope);
		expect(scopes).not.toContain('company');
		expect(scopes).not.toContain('client');
		expect(scopes).toEqual(['personal', 'project', 'stack']);
	});

	it('has no coverage/autonomy conflict — nothing broader to override the personal preference', () => {
		expect(eff.conflicts.find((c) => c.topic === 'coverage')).toBeUndefined();
		expect(eff.conflicts.find((c) => c.topic === 'autonomy')).toBeUndefined();
	});
});

describe('agency monorepo (agency-monorepo) — two Client rungs, isolated', () => {
	const eff = resolveConstitution(buildLadder(projectById('mono')));

	it('has two distinct client rungs over one company base', () => {
		const clientRungs = eff.ladder.filter((r) => r.scope === 'client');
		expect(clientRungs.length).toBe(2);
		expect(clientRungs[0].label).toBe('Globex');
		expect(clientRungs[1].label).toBe('Initech');
		expect(eff.ladder.filter((r) => r.scope === 'company').length).toBe(1);
	});

	it('each client keeps its own isolation lock (no cross-client merge)', () => {
		for (const cr of eff.ladder.filter((r) => r.scope === 'client')) {
			const iso = cr.rules.find((r) => r.topic === 'isolation')!;
			expect(iso.status).toBe<RuleStatus>('non-negotiable');
			expect(iso.text).toContain(cr.label);
		}
	});
});

describe('reclassification — company ↔ client changes which rungs apply', () => {
	it('reclassifying a company project as client switches the Client rung ON', () => {
		const base = projectById('auth'); // company
		const asClient = { ...base, kind: 'client' as const };
		const before = resolveConstitution(buildLadder(base));
		const after = resolveConstitution(buildLadder(asClient));
		expect(before.ladder.some((r) => r.scope === 'client')).toBe(false);
		expect(after.ladder.some((r) => r.scope === 'client')).toBe(true);
		// The company coverage lock survives the reclassification (a lock is
		// scope-independent — reclassifying changes which RULES apply, not the lock).
		expect(after.conflicts.find((c) => c.topic === 'coverage')?.locked).toBe(true);
	});

	it('reclassifying a client project as company switches the Client rung OFF', () => {
		const base = projectById('globex'); // client
		const asCompany = { ...base, kind: 'company' as const, clients: undefined };
		const before = resolveConstitution(buildLadder(base));
		const after = resolveConstitution(buildLadder(asCompany));
		expect(before.ladder.some((r) => r.scope === 'client')).toBe(true);
		expect(after.ladder.some((r) => r.scope === 'client')).toBe(false);
	});
});

describe('helper counts', () => {
	it('totalRuleCount counts winners across an arbitrary ladder', () => {
		const ladder: ResolvedRung[] = resolveConstitution(refineLadder()).ladder;
		expect(totalRuleCount(ladder)).toBe(1); // one topic, one winner
	});

	it('lockedCount counts non-negotiable winners', () => {
		const ladder: ResolvedRung[] = resolveConstitution(lockLadder()).ladder;
		expect(lockedCount(ladder)).toBe(1);
	});
});
