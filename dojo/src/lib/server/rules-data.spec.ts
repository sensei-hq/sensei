// Unit tests for the rules-federation store logic (`rules-data.ts`) — the TS
// port of dojo-mind's `DojoStore` publish/pull/retract. Exercises:
//   • parsePublishedRule — required-field validation, optional null-coercion,
//     and that the client's published_by/published_at are NOT read.
//   • shapePullResponse — the PullResponse wire shape (flattened PulledRule,
//     embedded-namespace normalization) + cursor = max(seq) / since-when-empty.
//   • publishRule — namespace upsert (sensei schema, (scope_key,slug)) then
//     shared_rules upsert ((namespace_id,content_hash)), server-controlled
//     attribution, {id,version,seq} return, error → RulesError(500).
//   • retractRule — status→tombstoned with the non-tombstoned guard, boolean
//     result, error → RulesError(500).
//   • recordRulesAudit — the audit_events insert shape; a failed write is
//     logged, not thrown (non-fatal but never silent).
// A chainable supabase-js stub (no live DB), like the sibling route specs.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
	parsePublishedRule,
	shapePullResponse,
	publishRule,
	pullRules,
	retractRule,
	recordRulesAudit,
	RulesError,
	PULL_SELECT,
	maxTier,
	effectivePackRuleTier,
	parseNamespacePairs,
	composeAdoptedPackRules,
	type DojoClient
} from './rules-data';

type Terminal = { data: unknown; error: unknown };
interface Call {
	schema?: string;
	table?: string;
	op?: string;
	payload?: unknown;
	onConflict?: string;
	select?: string;
	filters: [string, string, unknown][];
	order?: [string, unknown];
}
function makeDb() {
	const calls: Call[] = [];
	let results: Terminal[] = [];
	// `pendingSchema` is set only by an immediately-preceding `.schema(...)`; a
	// bare `.from(...)` on the base client (no `.schema()` first) is base-scoped
	// (undefined) — matching supabase-js, where the dojo-scoped base client's
	// `.from()` stays on `dojo` unless `.schema('sensei')` re-scopes that chain.
	let pendingSchema: string | undefined;
	let cur: Call;
	const b: Record<string, unknown> = {};
	b.schema = (s: string) => {
		pendingSchema = s;
		return b;
	};
	b.from = (t: string) => {
		cur = { schema: pendingSchema, table: t, filters: [] };
		pendingSchema = undefined;
		calls.push(cur);
		return b;
	};
	b.select = (s?: string) => {
		if (cur) cur.select = s;
		return b;
	};
	b.eq = (c: string, v: unknown) => {
		cur.filters.push(['eq', c, v]);
		return b;
	};
	b.neq = (c: string, v: unknown) => {
		cur.filters.push(['neq', c, v]);
		return b;
	};
	b.gt = (c: string, v: unknown) => {
		cur.filters.push(['gt', c, v]);
		return b;
	};
	b.order = (c: string, o: unknown) => {
		cur.order = [c, o];
		return b;
	};
	b.insert = (p: unknown) => {
		cur.op = 'insert';
		cur.payload = p;
		return b;
	};
	b.update = (p: unknown) => {
		cur.op = 'update';
		cur.payload = p;
		return b;
	};
	b.upsert = (p: unknown, opts?: { onConflict?: string }) => {
		cur.op = 'upsert';
		cur.payload = p;
		cur.onConflict = opts?.onConflict;
		return b;
	};
	b.single = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	b.maybeSingle = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	// Awaiting the builder (the pull list query) resolves the next queued result.
	b.then = (resolve: (v: Terminal) => unknown) => resolve(results.shift() ?? { data: [], error: null });
	return {
		client: b as unknown as DojoClient,
		calls,
		queue(...r: Terminal[]) {
			results = r;
		}
	};
}

const RULE = {
	content_hash: 'a'.repeat(64),
	scope_key: 'organization',
	namespace_slug: 'sensei-hq',
	namespace_name: 'Sensei HQ',
	rule_type: 'convention',
	title: 'Always use TDD',
	content: 'Always use TDD.',
	impact: null,
	enforcement: 'mandatory',
	origin_repo: 'sensei/daemon'
};

describe('parsePublishedRule', () => {
	it('accepts a well-formed body and coerces optional fields to null', () => {
		const r = parsePublishedRule({ ...RULE, impact: undefined, origin_repo: undefined });
		expect(r).not.toBeNull();
		expect(r?.impact).toBeNull();
		expect(r?.origin_repo).toBeNull();
		expect(r?.content_hash).toBe('a'.repeat(64));
	});

	it('preserves present optional fields', () => {
		const r = parsePublishedRule({ ...RULE, impact: 'high', origin_repo: 'sensei/app' });
		expect(r?.impact).toBe('high');
		expect(r?.origin_repo).toBe('sensei/app');
	});

	it('does NOT surface the client published_by / published_at (server-controlled)', () => {
		const r = parsePublishedRule({ ...RULE, published_by: 'attacker', published_at: '2000-01-01T00:00:00Z' });
		expect(r).not.toHaveProperty('published_by');
		expect(r).not.toHaveProperty('published_at');
	});

	for (const missing of [
		'content_hash',
		'scope_key',
		'namespace_slug',
		'namespace_name',
		'rule_type',
		'title',
		'content',
		'enforcement'
	]) {
		it(`returns null when required field '${missing}' is absent`, () => {
			const body: Record<string, unknown> = { ...RULE };
			delete body[missing];
			expect(parsePublishedRule(body)).toBeNull();
		});
		it(`returns null when required field '${missing}' is blank`, () => {
			expect(parsePublishedRule({ ...RULE, [missing]: '   ' })).toBeNull();
		});
	}
});

describe('shapePullResponse (PullResponse wire shape + cursor)', () => {
	const row = (seq: number, status = 'active') => ({
		id: `rule-${seq}`,
		seq,
		status,
		version: 1,
		content_hash: 'a'.repeat(64),
		rule_type: 'convention',
		title: 'T',
		content: 'c',
		impact: null,
		enforcement: 'mandatory',
		origin_repo: null,
		published_by: 'jerry',
		published_at: '2026-07-08T00:00:00Z',
		namespaces: { scope_key: 'organization', slug: 'sensei-hq', name: 'Sensei HQ' }
	});

	it('flattens each row into a PulledRule (id/seq/status/version + PublishedRule fields)', () => {
		const out = shapePullResponse([row(5)], 0);
		expect(out.rules).toHaveLength(1);
		const r = out.rules[0];
		expect(r.id).toBe('rule-5');
		expect(r.seq).toBe(5);
		expect(r.status).toBe('active');
		expect(r.version).toBe(1);
		expect(r.scope_key).toBe('organization');
		expect(r.namespace_slug).toBe('sensei-hq');
		expect(r.namespace_name).toBe('Sensei HQ');
		// The nested `namespaces` embed must not leak onto the wire object.
		expect(r).not.toHaveProperty('namespaces');
	});

	it('normalizes a one-element-array namespace embed (PostgREST array shape)', () => {
		const r = { ...row(3), namespaces: [{ scope_key: 'project', slug: 'sensei', name: 'sensei' }] };
		const out = shapePullResponse([r], 0);
		expect(out.rules[0].scope_key).toBe('project');
		expect(out.rules[0].namespace_slug).toBe('sensei');
	});

	it('computes cursor = max(seq)', () => {
		const out = shapePullResponse([row(5), row(9), row(7)], 3);
		expect(out.cursor).toBe(9);
	});

	it('empty page → cursor = since', () => {
		expect(shapePullResponse([], 42).cursor).toBe(42);
		expect(shapePullResponse([], 0).cursor).toBe(0);
	});

	it('carries the tombstoned status through unchanged', () => {
		const out = shapePullResponse([row(9, 'tombstoned')], 0);
		expect(out.rules[0].status).toBe('tombstoned');
	});
});

describe('publishRule', () => {
	it('upserts the namespace (sensei schema) then the rule, returning {id,version,seq}', async () => {
		const db = makeDb();
		db.queue(
			{ data: { id: 'ns-1' }, error: null },
			{ data: { id: 'rule-1', version: 1, seq: 7 }, error: null }
		);
		const resp = await publishRule(db.client, RULE, 'user-uuid');
		expect(resp).toEqual({ id: 'rule-1', version: 1, seq: 7 });

		const ns = db.calls.find((c) => c.table === 'namespaces')!;
		expect(ns.schema).toBe('sensei');
		expect(ns.op).toBe('upsert');
		expect(ns.onConflict).toBe('scope_key,slug');
		expect(ns.payload).toMatchObject({ scope_key: 'organization', slug: 'sensei-hq', name: 'Sensei HQ' });

		const rule = db.calls.find((c) => c.table === 'shared_rules')!;
		expect(rule.op).toBe('upsert');
		expect(rule.onConflict).toBe('namespace_id,content_hash');
		// shared_rules is in the `dojo` schema (the base client) — NOT re-scoped to sensei.
		expect(rule.schema).toBeUndefined();
		const p = rule.payload as Record<string, unknown>;
		expect(p.namespace_id).toBe('ns-1');
		expect(p.content_hash).toBe('a'.repeat(64));
		expect(p.status).toBe('active');
		expect(p.enforcement).toBe('mandatory');
	});

	it('stamps published_by from the caller and published_at server-side (not the body)', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'ns-1' }, error: null }, { data: { id: 'r', version: 1, seq: 1 }, error: null });
		await publishRule(db.client, RULE, 'caller-uuid');
		const p = db.calls.find((c) => c.table === 'shared_rules')!.payload as Record<string, unknown>;
		expect(p.published_by).toBe('caller-uuid');
		expect(typeof p.published_at).toBe('string');
		// published_at is a real ISO instant, not an epoch placeholder.
		expect(p.published_at).not.toBe('1970-01-01T00:00:00Z');
		expect(p.updated_at).toBe(p.published_at);
	});

	it('throws RulesError(500) when the namespace upsert errors (never silent)', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'ns boom' } });
		await expect(publishRule(db.client, RULE, 'u')).rejects.toMatchObject({
			status: 500,
			message: 'ns boom'
		});
		expect(RulesError).toBeDefined();
	});

	it('throws RulesError(500) when the rule upsert errors', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'ns-1' }, error: null }, { data: null, error: { message: 'rule boom' } });
		await expect(publishRule(db.client, RULE, 'u')).rejects.toMatchObject({ status: 500 });
	});
});

describe('pullRules', () => {
	it('filters seq > since, orders by seq asc, selects the join, and shapes the response', async () => {
		const db = makeDb();
		db.queue({
			data: [
				{
					id: 'rule-5',
					seq: 5,
					status: 'active',
					version: 1,
					content_hash: 'a'.repeat(64),
					rule_type: 'convention',
					title: 'T',
					content: 'c',
					impact: null,
					enforcement: 'mandatory',
					origin_repo: null,
					published_by: 'jerry',
					published_at: '2026-07-08T00:00:00Z',
					namespaces: { scope_key: 'organization', slug: 'sensei-hq', name: 'Sensei HQ' }
				}
			],
			error: null
		});
		const out = await pullRules(db.client, 3);
		expect(out.cursor).toBe(5);
		expect(out.rules[0].scope_key).toBe('organization');

		const call = db.calls.find((c) => c.table === 'shared_rules')!;
		expect(call.select).toBe(PULL_SELECT);
		expect(call.filters).toContainEqual(['gt', 'seq', 3]);
		expect(call.order).toEqual(['seq', { ascending: true }]);
	});

	it('empty result → { rules: [], cursor: since }', async () => {
		const db = makeDb();
		db.queue({ data: [], error: null });
		const out = await pullRules(db.client, 42);
		expect(out).toEqual({ rules: [], cursor: 42 });
	});

	it('throws RulesError(500) on a select error', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'pull boom' } });
		await expect(pullRules(db.client, 0)).rejects.toMatchObject({ status: 500, message: 'pull boom' });
	});
});

describe('retractRule', () => {
	it('sets status→tombstoned with the non-tombstoned guard and returns true on a hit', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'rule-1' }, error: null });
		const ok = await retractRule(db.client, 'rule-1');
		expect(ok).toBe(true);
		const call = db.calls.find((c) => c.table === 'shared_rules')!;
		expect(call.op).toBe('update');
		expect((call.payload as Record<string, unknown>).status).toBe('tombstoned');
		expect(call.filters).toContainEqual(['eq', 'id', 'rule-1']);
		expect(call.filters).toContainEqual(['neq', 'status', 'tombstoned']);
	});

	it('returns false when no active row matched (already tombstoned / unknown id)', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null });
		expect(await retractRule(db.client, 'rule-x')).toBe(false);
	});

	it('throws RulesError(500) on an update error', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'retract boom' } });
		await expect(retractRule(db.client, 'r')).rejects.toMatchObject({ status: 500 });
	});
});

describe('recordRulesAudit', () => {
	it('inserts a tenant-scoped audit_events row with actor/action/target/detail', async () => {
		const db = makeDb();
		db.queue({ data: { id: 1 }, error: null });
		await recordRulesAudit(db.client, 't1', 'publish', 'rule-1', 'actor-uuid', { version: 2, seq: 9 });
		const call = db.calls.find((c) => c.table === 'audit_events')!;
		expect(call.op).toBe('insert');
		const p = call.payload as Record<string, unknown>;
		expect(p.tenant_id).toBe('t1');
		expect(p.actor_id).toBe('actor-uuid');
		expect(p.action).toBe('publish');
		expect(p.target).toBe('rule-1');
		expect(p.detail).toEqual({ version: 2, seq: 9 });
	});

	it('logs (does not throw) when the audit insert fails — non-fatal but never silent', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'audit boom' } });
		const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
		await expect(recordRulesAudit(db.client, 't1', 'retract', 'rule-1', 'a')).resolves.toBeUndefined();
		expect(errSpy).toHaveBeenCalledWith(expect.stringContaining('audit boom'));
		errSpy.mockRestore();
	});

	it('defaults detail to {}', async () => {
		const db = makeDb();
		db.queue({ data: { id: 1 }, error: null });
		await recordRulesAudit(db.client, 't1', 'retract', 'rule-1', 'a');
		const p = db.calls.find((c) => c.table === 'audit_events')!.payload as Record<string, unknown>;
		expect(p.detail).toEqual({});
	});
});

describe('rule-pack tier precedence', () => {
	it('maxTier returns the stronger enforcement tier', () => {
		expect(maxTier('advisory', 'required')).toBe('required');
		expect(maxTier('mandatory', 'required')).toBe('mandatory');
		expect(maxTier('recommended', 'recommended')).toBe('recommended');
	});

	it('an adoption override raises a rule but never weakens it', () => {
		// adopt an advisory pack as required → the rule is raised
		expect(effectivePackRuleTier('advisory', 'required')).toBe('required');
		// a mandatory rule can never be lowered by a weaker override
		expect(effectivePackRuleTier('mandatory', 'required')).toBe('mandatory');
		// no override → the rule keeps its own tier
		expect(effectivePackRuleTier('required', null)).toBe('required');
	});

	it('parseNamespacePairs splits scope:slug and drops malformed', () => {
		expect(parseNamespacePairs('organization:acme, stack:react')).toEqual([
			{ scope_key: 'organization', slug: 'acme' },
			{ scope_key: 'stack', slug: 'react' }
		]);
		// slugs may contain colons (only the first splits scope from slug)
		expect(parseNamespacePairs('project:acme/lumen-auth')).toEqual([
			{ scope_key: 'project', slug: 'acme/lumen-auth' }
		]);
		expect(parseNamespacePairs('')).toEqual([]);
		expect(parseNamespacePairs('garbage,,:x,y:')).toEqual([]);
	});
});

describe('composeAdoptedPackRules — pack rules carry their ADOPTION scope', () => {
	const packs = [{ id: 'pk1', area: 'security', source: 'OWASP' }];
	const rules = [
		{ id: 'r1', pack_id: 'pk1', ordinal: 1, statement: 'S1', body: 'B1', rationale: 'why', enforcement: 'advisory', verification: 'manual', checker_ref: null, remediation: null, skill_ref: null, applies_to: null },
		{ id: 'r2', pack_id: 'pk1', ordinal: 2, statement: 'S2', body: 'B2', rationale: null, enforcement: 'mandatory', verification: 'manual', checker_ref: null, remediation: null, skill_ref: null, applies_to: null }
	];

	it('scopes a rule by the adoption namespace scope_key, NOT the pack area', () => {
		const out = composeAdoptedPackRules(
			[{ pack_id: 'pk1', namespace_id: 'ns-org', enforcement: null }],
			[{ id: 'ns-org', scope_key: 'organization' }],
			packs,
			rules
		);
		expect(out.map((r) => r.scope_key)).toEqual(['organization', 'organization']);
		expect(out.every((r) => r.scope_key !== 'security')).toBe(true); // never the pack area
		expect(out.map((r) => r.statement)).toEqual(['S1', 'S2']); // ordinal order
		expect(out[0].source).toBe('OWASP');
	});

	it('applies the adoption override never-weaken (raises advisory, keeps mandatory)', () => {
		const out = composeAdoptedPackRules(
			[{ pack_id: 'pk1', namespace_id: 'ns-org', enforcement: 'required' }],
			[{ id: 'ns-org', scope_key: 'organization' }],
			packs,
			rules
		);
		expect(out.find((r) => r.statement === 'S1')?.enforcement).toBe('required'); // advisory → required
		expect(out.find((r) => r.statement === 'S2')?.enforcement).toBe('mandatory'); // not weakened
	});

	it('emits a rule per adopting namespace, each at its own scope (daemon dedups downstream)', () => {
		const out = composeAdoptedPackRules(
			[
				{ pack_id: 'pk1', namespace_id: 'ns-org', enforcement: null },
				{ pack_id: 'pk1', namespace_id: 'ns-proj', enforcement: null }
			],
			[
				{ id: 'ns-org', scope_key: 'organization' },
				{ id: 'ns-proj', scope_key: 'project' }
			],
			packs,
			rules
		);
		// r1 + r2 at organization, r1 + r2 at project.
		expect(out.filter((r) => r.scope_key === 'organization').length).toBe(2);
		expect(out.filter((r) => r.scope_key === 'project').length).toBe(2);
	});

	it('skips an adoption whose namespace is not in the resolved set', () => {
		const out = composeAdoptedPackRules(
			[{ pack_id: 'pk1', namespace_id: 'ns-unknown', enforcement: null }],
			[{ id: 'ns-org', scope_key: 'organization' }],
			packs,
			rules
		);
		expect(out).toEqual([]);
	});
});
