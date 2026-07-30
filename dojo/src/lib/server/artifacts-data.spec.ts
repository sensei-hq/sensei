// Unit tests for the artifact-federation store logic (`artifacts-data.ts`) — the
// TS port of dojo-mind's `DojoStore::publish_artifact` / `pull_artifacts_since`
// plus the INLINE `promote_cluster` (collective/promote.rs). Exercises:
//   • the PURE promote math (verbatim from promote.rs, no DB): score / decide /
//     the k-anonymity gate / distinctContributors / bestFtrDelta.
//   • parsePublishedArtifact — required-field validation, kind↔payload match,
//     attribution validation, optional null-coercion, and that the client's
//     contributed_by / published_at are NOT read (server-controlled).
//   • shapeArtifactPullResponse — the ArtifactPullResponse wire shape (flattened
//     PulledArtifact, embedded-tenant normalization) + cursor = max(seq)/since.
//   • publishArtifact — insert into dojo.artifacts (server-controlled
//     contributed_by), {id,seq} return, error → ArtifactError(500).
//   • pullArtifactsSince — tenant scope + seq>since + order + shape.
//   • promoteCluster — no-op / merge / auto-approve / queue paths, k-anonymity,
//     content_hash (signature) clustering, seq via the artifacts_next_seq RPC.
// A chainable supabase-js stub (no live DB), like the sibling `rules-data.spec.ts`.
import { describe, it, expect } from 'vitest';
import {
	score,
	decide,
	distinctContributors,
	bestFtrDelta,
	parsePublishedArtifact,
	shapeArtifactPullResponse,
	publishArtifact,
	pullArtifactsSince,
	promoteCluster,
	ArtifactError,
	ARTIFACT_PULL_SELECT,
	AUTO_APPROVE_SCORE,
	PER_CONTRIBUTOR_WEIGHT,
	FTR_DELTA_FULL_CREDIT,
	K_ANONYMITY_MIN_CONTRIBUTORS,
	type ArtifactRow,
	type ClusterSignals,
	type DojoClient
} from './artifacts-data';

// ── chainable supabase-js stub ───────────────────────────────────────────────

type Terminal = { data: unknown; error: unknown };
interface Call {
	table?: string;
	op?: string;
	payload?: unknown;
	select?: string;
	filters: [string, string, unknown][];
	order?: [string, unknown];
	limit?: number;
}
function makeDb() {
	const calls: Call[] = [];
	let results: Terminal[] = [];
	let cur: Call;
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		cur = { table: t, filters: [] };
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
	b.limit = (n: number) => {
		cur.limit = n;
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
	b.single = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	b.maybeSingle = () => Promise.resolve(results.shift() ?? { data: null, error: null });
	// Awaiting the builder (list query or a filtered update with no .single())
	// resolves the next queued result — default an empty list so list selects and
	// terminal updates both work.
	b.then = (resolve: (v: Terminal) => unknown) =>
		resolve(results.shift() ?? { data: [], error: null });
	return {
		client: b as unknown as DojoClient,
		calls,
		queue(...r: Terminal[]) {
			results = r;
		}
	};
}

const signals = (
	contributorCount: number,
	ftrDeltaObserved: number | null,
	isGlobal: boolean
): ClusterSignals => ({ contributorCount, ftrDeltaObserved, isGlobal });

// ── pure promote math (mirrors promote.rs's unit tests exactly) ──────────────

describe('score (promote.rs::score)', () => {
	it('breadth-only: five contributors hits the bar; four fall short', () => {
		expect(score(signals(5, null, false))).toBeCloseTo(0.8, 9);
		expect(score(signals(5, null, false))).toBeGreaterThanOrEqual(AUTO_APPROVE_SCORE);
		expect(score(signals(4, null, false))).toBeLessThan(AUTO_APPROVE_SCORE);
	});

	it('efficacy: a single strong contribution can clear the bar', () => {
		const s = score(signals(1, FTR_DELTA_FULL_CREDIT, false));
		expect(s).toBeCloseTo(1.0, 9);
		expect(s).toBeGreaterThanOrEqual(AUTO_APPROVE_SCORE);
	});

	it('is clamped to 1.0 and ignores negative FTR', () => {
		expect(score(signals(100, 10.0, false))).toBe(1.0);
		expect(score(signals(1, -0.5, false))).toBeCloseTo(score(signals(1, null, false)), 9);
		// Below-zero contributor counts can't push the score negative.
		expect(score(signals(-3, null, false))).toBe(0.0);
	});

	it('the constants match promote.rs', () => {
		expect(AUTO_APPROVE_SCORE).toBe(0.8);
		expect(PER_CONTRIBUTOR_WEIGHT).toBe(0.16);
		expect(FTR_DELTA_FULL_CREDIT).toBe(0.2);
		expect(K_ANONYMITY_MIN_CONTRIBUTORS).toBe(3);
	});
});

describe('decide (promote.rs::decide — k-anonymity gate)', () => {
	it('low bar, single contributor → queued (below score bar)', () => {
		expect(decide(signals(1, null, false))).toEqual({ kind: 'queue', reason: 'below_score_bar' });
	});

	it('private tenant, single strong contribution → auto-approve (no k-anon floor)', () => {
		expect(decide(signals(1, FTR_DELTA_FULL_CREDIT, false))).toEqual({ kind: 'auto_approve' });
	});

	it('five contributors → auto-approve', () => {
		expect(decide(signals(5, null, false))).toEqual({ kind: 'auto_approve' });
	});

	it('global under K is blocked even with a high score', () => {
		// score clears (0.16 + full efficacy = 1.0) but 1 < K=3 → k-anonymity hold.
		expect(decide(signals(1, FTR_DELTA_FULL_CREDIT, true))).toEqual({
			kind: 'queue',
			reason: 'k_anonymity'
		});
		expect(decide(signals(2, FTR_DELTA_FULL_CREDIT, true))).toEqual({
			kind: 'queue',
			reason: 'k_anonymity'
		});
	});

	it('global at K (=3) with a high score → auto-approve', () => {
		expect(decide(signals(3, FTR_DELTA_FULL_CREDIT, true))).toEqual({ kind: 'auto_approve' });
	});

	it('global that also fails the score bar reports below_score_bar, not k_anonymity', () => {
		expect(decide(signals(1, null, true))).toEqual({ kind: 'queue', reason: 'below_score_bar' });
	});
});

describe('distinctContributors (count(distinct coalesce(contributed_by, anon_id)))', () => {
	const member = (contributed_by: string | null, anonymous_id: string | null) => ({
		id: 'x',
		seq: 1,
		scope: {},
		contributed_by,
		attribution: anonymous_id
			? { mode: 'anonymous' as const, anonymous_id }
			: null,
		payload: null
	});

	it('counts distinct user ids', () => {
		expect(distinctContributors([member('u1', null), member('u2', null), member('u1', null)])).toBe(2);
	});

	it('falls back to the anonymous_id when contributed_by is null', () => {
		expect(distinctContributors([member(null, 'anon-a'), member(null, 'anon-b'), member(null, 'anon-a')])).toBe(2);
	});

	it('prefers contributed_by over anonymous_id (coalesce order)', () => {
		// Same person: once named, once anon — coalesce takes contributed_by first,
		// so these are two DISTINCT keys (u1 vs anon-1), matching the SQL coalesce.
		expect(distinctContributors([member('u1', null), member(null, 'anon-1')])).toBe(2);
	});

	it('ignores members with neither identity (NULL coalesce, like count(distinct))', () => {
		expect(distinctContributors([member(null, null), member('u1', null)])).toBe(1);
	});
});

describe('bestFtrDelta (max(payload->>ftr_delta_observed))', () => {
	const m = (ftr: number | string | null) => ({
		id: 'x',
		seq: 1,
		scope: {},
		contributed_by: null,
		attribution: null,
		payload: ftr == null ? {} : { ftr_delta_observed: ftr }
	});
	it('returns the max across members', () => {
		expect(bestFtrDelta([m(0.05), m(0.2), m(0.1)])).toBeCloseTo(0.2, 9);
	});
	it('coerces string payload values (jsonb ->> is text)', () => {
		expect(bestFtrDelta([m('0.07'), m('0.03')])).toBeCloseTo(0.07, 9);
	});
	it('null when no member carries a delta', () => {
		expect(bestFtrDelta([m(null), m(null)])).toBeNull();
	});
});

// ── parsePublishedArtifact ───────────────────────────────────────────────────

const ARTIFACT = {
	signature: 'a'.repeat(64),
	tenant_key: 'github/sensei-hq',
	engagement_id: null,
	kind: 'pattern',
	title: 'Prefer adapters',
	body: 'Wrap the vendor SDK behind an adapter.',
	payload: { kind: 'pattern', family: 'design', pattern_id: 'codebase.adapter', ftr_delta_observed: 0.07 },
	scope: { stack: 'rust' },
	attribution: { mode: 'anonymous', anonymous_id: 'anon-1' }
};

describe('parsePublishedArtifact', () => {
	it('accepts a well-formed body', () => {
		const a = parsePublishedArtifact({ ...ARTIFACT });
		expect(a).not.toBeNull();
		expect(a?.kind).toBe('pattern');
		expect(a?.signature).toBe('a'.repeat(64));
		expect(a?.attribution.anonymous_id).toBe('anon-1');
	});

	it('coerces optional fields to null', () => {
		const a = parsePublishedArtifact({ ...ARTIFACT, engagement_id: undefined });
		expect(a?.engagement_id).toBeNull();
	});

	it('defaults scope to {} when absent', () => {
		const a = parsePublishedArtifact({ ...ARTIFACT, scope: undefined });
		expect(a?.scope).toEqual({});
	});

	it('does NOT surface the client contributed_by / published_at (server-controlled)', () => {
		const a = parsePublishedArtifact({
			...ARTIFACT,
			contributed_by: 'attacker',
			published_at: '2000-01-01T00:00:00Z'
		});
		expect(a).not.toHaveProperty('contributed_by');
		expect(a).not.toHaveProperty('published_at');
	});

	it('rejects a payload whose kind tag disagrees with the envelope kind', () => {
		expect(parsePublishedArtifact({ ...ARTIFACT, payload: { kind: 'principle' } })).toBeNull();
	});

	it('rejects an unknown kind', () => {
		expect(parsePublishedArtifact({ ...ARTIFACT, kind: 'bogus', payload: { kind: 'bogus' } })).toBeNull();
	});

	for (const missing of ['signature', 'tenant_key', 'title', 'body']) {
		it(`returns null when required field '${missing}' is absent`, () => {
			const body: Record<string, unknown> = { ...ARTIFACT };
			delete body[missing];
			expect(parsePublishedArtifact(body)).toBeNull();
		});
		it(`returns null when required field '${missing}' is blank`, () => {
			expect(parsePublishedArtifact({ ...ARTIFACT, [missing]: '   ' })).toBeNull();
		});
	}

	it('rejects a missing/invalid attribution', () => {
		expect(parsePublishedArtifact({ ...ARTIFACT, attribution: undefined })).toBeNull();
		expect(parsePublishedArtifact({ ...ARTIFACT, attribution: { mode: 'bogus' } })).toBeNull();
	});
});

// ── shapeArtifactPullResponse ────────────────────────────────────────────────

describe('shapeArtifactPullResponse (ArtifactPullResponse wire shape + cursor)', () => {
	const row = (seq: number, status = 'published'): ArtifactRow => ({
		id: `art-${seq}`,
		seq,
		status,
		engagement_id: null,
		kind: 'principle',
		title: 'T',
		body: 'b',
		payload: { kind: 'principle' },
		scope: { stack: 'rust' },
		signature: 'a'.repeat(64),
		attribution: { mode: 'anonymous', anonymous_id: 'anon-1' },
		contributed_by: null,
		published_at: '2026-07-08T00:00:00Z',
		tenants: { key: 'github/sensei-hq' }
	});

	it('flattens each row into a PulledArtifact (id/seq/status + PublishedArtifact fields)', () => {
		const out = shapeArtifactPullResponse([row(5)], 0);
		expect(out.artifacts).toHaveLength(1);
		const a = out.artifacts[0];
		expect(a.id).toBe('art-5');
		expect(a.seq).toBe(5);
		expect(a.status).toBe('published');
		expect(a.tenant_key).toBe('github/sensei-hq');
		expect(a.kind).toBe('principle');
		// The nested `tenants` embed must not leak onto the wire object.
		expect(a).not.toHaveProperty('tenants');
		// No `version` field on artifacts (unlike rules).
		expect(a).not.toHaveProperty('version');
	});

	it('normalizes a one-element-array tenant embed (PostgREST array shape)', () => {
		const r = { ...row(3), tenants: [{ key: 'org/global-dojo' }] };
		expect(shapeArtifactPullResponse([r], 0).artifacts[0].tenant_key).toBe('org/global-dojo');
	});

	it('computes cursor = max(seq)', () => {
		expect(shapeArtifactPullResponse([row(5), row(9), row(7)], 3).cursor).toBe(9);
	});

	it('empty page → cursor = since', () => {
		expect(shapeArtifactPullResponse([], 42).cursor).toBe(42);
	});

	it('carries submitted / archived statuses through unchanged', () => {
		expect(shapeArtifactPullResponse([row(9, 'submitted')], 0).artifacts[0].status).toBe('submitted');
		expect(shapeArtifactPullResponse([row(9, 'archived')], 0).artifacts[0].status).toBe('archived');
	});
});

// ── publishArtifact ──────────────────────────────────────────────────────────

describe('publishArtifact', () => {
	it('inserts into dojo.artifacts (status defaulted) and returns {id,seq}', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'art-1', seq: 7 }, error: null });
		const artifact = parsePublishedArtifact({ ...ARTIFACT })!;
		const resp = await publishArtifact(db.client, 't1', artifact, 'caller-uuid');
		expect(resp).toEqual({ id: 'art-1', seq: 7 });

		const call = db.calls.find((c) => c.table === 'artifacts')!;
		expect(call.op).toBe('insert');
		const p = call.payload as Record<string, unknown>;
		expect(p.tenant_id).toBe('t1');
		expect(p.signature).toBe('a'.repeat(64));
		expect(p.kind).toBe('pattern');
		// contributed_by comes from the caller, not the body.
		expect(p.contributed_by).toBe('caller-uuid');
		// status is left to the DDL default (submitted) — not set explicitly.
		expect(p).not.toHaveProperty('status');
		// seq is left to the column default (nextval) — not set on insert.
		expect(p).not.toHaveProperty('seq');
	});

	it('records a null contributor when anonymised', async () => {
		const db = makeDb();
		db.queue({ data: { id: 'art-1', seq: 1 }, error: null });
		await publishArtifact(db.client, 't1', parsePublishedArtifact({ ...ARTIFACT })!, null);
		const p = db.calls.find((c) => c.table === 'artifacts')!.payload as Record<string, unknown>;
		expect(p.contributed_by).toBeNull();
	});

	it('throws ArtifactError(500) on an insert error (never silent)', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'insert boom' } });
		await expect(
			publishArtifact(db.client, 't1', parsePublishedArtifact({ ...ARTIFACT })!, 'u')
		).rejects.toMatchObject({ status: 500, message: 'insert boom' });
		expect(ArtifactError).toBeDefined();
	});
});

// ── pullArtifactsSince ───────────────────────────────────────────────────────

describe('pullArtifactsSince', () => {
	it('scopes to the tenant, filters seq>since, orders asc, selects the join, shapes the response', async () => {
		const db = makeDb();
		db.queue({
			data: [
				{
					id: 'art-5',
					seq: 5,
					status: 'published',
					engagement_id: null,
					kind: 'guard',
					title: 'T',
					body: 'b',
					payload: { kind: 'guard', check: 'no unwrap' },
					scope: {},
					signature: 'a'.repeat(64),
					attribution: { mode: 'anonymous', anonymous_id: 'anon-1' },
					contributed_by: null,
					published_at: '2026-07-08T00:00:00Z',
					tenants: { key: 'github/sensei-hq' }
				}
			],
			error: null
		});
		const out = await pullArtifactsSince(db.client, 't1', 3);
		expect(out.cursor).toBe(5);
		expect(out.artifacts[0].tenant_key).toBe('github/sensei-hq');
		expect(out.artifacts[0].kind).toBe('guard');

		const call = db.calls.find((c) => c.table === 'artifacts')!;
		expect(call.select).toBe(ARTIFACT_PULL_SELECT);
		expect(call.filters).toContainEqual(['eq', 'tenant_id', 't1']);
		expect(call.filters).toContainEqual(['gt', 'seq', 3]);
		expect(call.order).toEqual(['seq', { ascending: true }]);
	});

	it('empty result → { artifacts: [], cursor: since }', async () => {
		const db = makeDb();
		db.queue({ data: [], error: null });
		expect(await pullArtifactsSince(db.client, 't1', 42)).toEqual({ artifacts: [], cursor: 42 });
	});

	it('throws ArtifactError(500) on a select error', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'pull boom' } });
		await expect(pullArtifactsSince(db.client, 't1', 0)).rejects.toMatchObject({
			status: 500,
			message: 'pull boom'
		});
	});
});

// ── promoteCluster (the inline promote_cluster port) ─────────────────────────

// The promote runner issues a fixed sequence of DB round-trips; each test queues
// the terminal results in call order. Helpers build a submitted-cluster row set.
const clusterMember = (id: string, seq: number, contributed_by: string | null, ftr?: number) => ({
	id,
	seq,
	scope: { stack: 'rust' },
	contributed_by,
	attribution: contributed_by ? null : { mode: 'anonymous', anonymous_id: `anon-${id}` },
	payload: ftr == null ? { kind: 'pattern' } : { kind: 'pattern', ftr_delta_observed: ftr }
});

describe('promoteCluster', () => {
	it('no-op when the tenant is unknown', async () => {
		const db = makeDb();
		db.queue({ data: null, error: null }); // tenants.scope maybeSingle → not found
		expect(await promoteCluster(db.client, 't-missing', 'sig')).toEqual({ outcome: 'no_op' });
	});

	it('no-op when nothing is submitted under the signature (idempotent)', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'private' }, error: null }, // tenant scope
			{ data: [], error: null } // empty submitted cluster
		);
		expect(await promoteCluster(db.client, 't1', 'sig')).toEqual({ outcome: 'no_op' });
	});

	it('auto-approves a private cluster of five contributors: publishes rep, archives rest, records approval', async () => {
		const db = makeDb();
		const members = [
			clusterMember('rep', 1, 'u1'),
			clusterMember('m2', 2, 'u2'),
			clusterMember('m3', 3, 'u3'),
			clusterMember('m4', 4, 'u4'),
			clusterMember('m5', 5, 'u5')
		];
		db.queue(
			{ data: { scope: 'private' }, error: null }, // tenant scope
			{ data: members, error: null }, // submitted cluster
			{ data: null, error: null }, // alreadyPublishedId → none
			{ data: { seq: 42 }, error: null }, // publishRepresentative update (insert-time seq)
			{ data: null, error: null }, // archiveSubmittedCluster (terminal update)
			{ data: null, error: null }, // upsertTriage select existing → none
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage insert
			{ data: null, error: null }, // insertDecision
			{ data: null, error: null } // insertEvent
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'published', artifact_id: 'rep', seq: 42 });

		// The published row is the representative (earliest seq) and status is guarded.
		const pub = db.calls.find(
			(c) => c.table === 'artifacts' && c.op === 'update' && (c.payload as Record<string, unknown>).status === 'published'
		)!;
		expect(pub.filters).toContainEqual(['eq', 'id', 'rep']);
		expect(pub.filters).toContainEqual(['eq', 'status', 'submitted']);
		// PostgREST can't advance seq inline, so the UPDATE patch does NOT set seq
		// (the published row keeps its insert-time seq — the documented divergence);
		// the returned seq (42) is what the insert stamped.
		expect(pub.payload).not.toHaveProperty('seq');

		// The rest of the cluster is archived, sparing the representative.
		const arch = db.calls.find(
			(c) => c.table === 'artifacts' && c.op === 'update' && (c.payload as Record<string, unknown>).status === 'archived'
		)!;
		expect(arch.filters).toContainEqual(['eq', 'status', 'submitted']);
		expect(arch.filters).toContainEqual(['neq', 'id', 'rep']);

		// An automated approval decision is recorded (maintainer null, automated true).
		const dec = db.calls.find((c) => c.table === 'decisions')!;
		const dp = dec.payload as Record<string, unknown>;
		expect(dp.status).toBe('approve');
		expect(dp.automated).toBe(true);
		expect(dp.maintainer_id).toBeNull();

		// The triage row records the auto_approved state.
		const triageIns = db.calls.find((c) => c.table === 'triage_queue' && c.op === 'insert')!;
		expect((triageIns.payload as Record<string, unknown>).state).toBe('auto_approved');

		const evt = db.calls.find((c) => c.table === 'events')!;
		expect((evt.payload as Record<string, unknown>).action).toBe('approved');
	});

	it('auto-approves a single strong contribution (efficacy) in a private tenant', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'private' }, error: null },
			{ data: [clusterMember('rep', 1, 'u1', FTR_DELTA_FULL_CREDIT)], error: null }, // 1 contributor + full efficacy → auto
			{ data: null, error: null }, // not already published
			{ data: { seq: 9 }, error: null }, // publishRepresentative → insert-time seq
			{ data: null, error: null }, // archive
			{ data: null, error: null }, // upsertTriage select
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage insert
			{ data: null, error: null }, // decision
			{ data: null, error: null } // event
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'published', artifact_id: 'rep', seq: 9 });
	});

	it('re-run no-ops when the representative is no longer submitted (concurrent promote won)', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'private' }, error: null },
			{ data: [clusterMember('rep', 1, 'u1', FTR_DELTA_FULL_CREDIT)], error: null },
			{ data: null, error: null }, // not already published
			{ data: null, error: null } // publishRepresentative update → no row matched status=submitted
		);
		expect(await promoteCluster(db.client, 't1', 'sig')).toEqual({ outcome: 'no_op' });
	});

	it('queues a below-bar private cluster and logs queued once (no publish)', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'private' }, error: null },
			{ data: [clusterMember('rep', 1, 'u1')], error: null }, // 1 contributor, no efficacy → below bar
			{ data: null, error: null }, // not already published
			{ data: null, error: null }, // upsertTriage select → none (newly inserted)
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage insert
			{ data: null, error: null } // insertEvent (queued)
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'queued', reason: 'below_score_bar' });

		// Nothing was published.
		expect(
			db.calls.find(
				(c) => c.op === 'update' && (c.payload as Record<string, unknown>).status === 'published'
			)
		).toBeUndefined();
		const triageIns = db.calls.find((c) => c.table === 'triage_queue' && c.op === 'insert')!;
		expect((triageIns.payload as Record<string, unknown>).state).toBe('queued');
		const evt = db.calls.find((c) => c.table === 'events')!;
		expect((evt.payload as Record<string, unknown>).action).toBe('queued');
		expect((evt.payload as Record<string, unknown>).detail).toMatchObject({ reason: 'below_score_bar' });
	});

	it('does NOT re-log queued when the triage row already exists (re-run)', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'private' }, error: null },
			{ data: [clusterMember('rep', 1, 'u1')], error: null },
			{ data: null, error: null }, // not already published
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage select → EXISTS
			{ data: null, error: null } // upsertTriage update
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'queued', reason: 'below_score_bar' });
		expect(db.calls.find((c) => c.table === 'events')).toBeUndefined();
	});

	it('holds a global cluster below K contributors on k-anonymity (queued, not published)', async () => {
		const db = makeDb();
		// 2 distinct contributors, both with full efficacy → score clears, but K=3.
		db.queue(
			{ data: { scope: 'global' }, error: null },
			{
				data: [clusterMember('rep', 1, 'u1', FTR_DELTA_FULL_CREDIT), clusterMember('m2', 2, 'u2')],
				error: null
			},
			{ data: null, error: null }, // not already published
			{ data: null, error: null }, // upsertTriage select → none
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage insert
			{ data: null, error: null } // insertEvent (queued)
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'queued', reason: 'k_anonymity' });
		expect(
			db.calls.find((c) => c.op === 'update' && (c.payload as Record<string, unknown>).status === 'published')
		).toBeUndefined();
		const evt = db.calls.find((c) => c.table === 'events')!;
		expect((evt.payload as Record<string, unknown>).detail).toMatchObject({ reason: 'k_anonymity' });
	});

	it('auto-approves a global cluster at K (=3) distinct contributors', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'global' }, error: null },
			{
				data: [
					clusterMember('rep', 1, 'u1', FTR_DELTA_FULL_CREDIT),
					clusterMember('m2', 2, 'u2'),
					clusterMember('m3', 3, 'u3')
				],
				error: null
			},
			{ data: null, error: null }, // not already published
			{ data: { seq: 50 }, error: null }, // publish update (insert-time seq)
			{ data: null, error: null }, // archive
			{ data: null, error: null }, // upsertTriage select
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage insert
			{ data: null, error: null }, // decision
			{ data: null, error: null } // event
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'published', artifact_id: 'rep', seq: 50 });
	});

	it('merges (archives) re-submissions of an already-published signature (content_hash dedup)', async () => {
		const db = makeDb();
		db.queue(
			{ data: { scope: 'private' }, error: null },
			{ data: [clusterMember('dup', 9, 'u9')], error: null }, // submitted re-submission
			{ data: { id: 'live-1' }, error: null }, // alreadyPublishedId → the live artifact
			{ data: null, error: null }, // archiveSubmittedCluster (all, except=null)
			{ data: null, error: null }, // upsertTriage select → none
			{ data: { id: 'triage-1' }, error: null }, // upsertTriage insert
			{ data: null, error: null } // insertEvent (merged)
		);
		const out = await promoteCluster(db.client, 't1', 'sig');
		expect(out).toEqual({ outcome: 'merged', into: 'live-1' });

		// The whole submitted cluster is archived (no representative spared → no neq filter).
		const arch = db.calls.find(
			(c) => c.table === 'artifacts' && c.op === 'update' && (c.payload as Record<string, unknown>).status === 'archived'
		)!;
		expect(arch.filters.find((f) => f[0] === 'neq')).toBeUndefined();
		const triageIns = db.calls.find((c) => c.table === 'triage_queue' && c.op === 'insert')!;
		expect((triageIns.payload as Record<string, unknown>).state).toBe('merged');
		const evt = db.calls.find((c) => c.table === 'events')!;
		expect((evt.payload as Record<string, unknown>).action).toBe('merged');
	});

	it('throws ArtifactError(500) when the tenant-scope lookup errors (never silent)', async () => {
		const db = makeDb();
		db.queue({ data: null, error: { message: 'scope boom' } });
		await expect(promoteCluster(db.client, 't1', 'sig')).rejects.toMatchObject({ status: 500 });
	});
});
