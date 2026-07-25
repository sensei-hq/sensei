// Pure/injectable data logic for the Worker rules-federation endpoints
// (`/v1/t/{origin}/{org}/rules`) — the TS port of dojo-mind's `DojoStore`
// publish/pull/retract (`crates/dojo-mind/src/store.rs`) + the audit write
// (`record_audit`). Kept out of `+server.ts` so the wire-shape logic (payload
// build, PullResponse shaping, cursor math) is unit-testable without a Worker.
//
// Wire contract: matches `dojo_protocol` (`crates/dojo-protocol/src/lib.rs`)
// byte-for-byte so senseid's federation client (`senseid/src/federation/mod.rs`)
// retargets with only a URL/auth swap:
//   • publish returns `PublishResponse { id, version, seq }`
//   • pull returns `PullResponse { rules: PulledRule[], cursor }`, where a
//     `PulledRule` is `{ id, seq, status, version, ...PublishedRule }`
//     (`#[serde(flatten)]` — the rule fields sit at the top level).
//
// dojo.shared_rules is NOT tenant-scoped (no tenant_id column — see the table's
// DDL comment and `seed_default_governance`): the registry is keyed on
// (namespace_id, content_hash). Tenant scoping is the AUTH boundary — the device
// token must resolve to a membership in the path tenant (`resolveApiKeyAccess`);
// the tenant is recorded on the `dojo.audit_events` row.

import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** The daemon's `PublishedRule` wire shape (`dojo_protocol::PublishedRule`). */
export interface PublishedRule {
	content_hash: string;
	scope_key: string;
	namespace_slug: string;
	namespace_name: string;
	rule_type: string;
	title: string;
	content: string;
	impact: string | null;
	enforcement: string;
	origin_repo: string | null;
	/** Client-supplied but IGNORED — attribution is server-controlled. */
	published_by?: string;
	/** Client-supplied but IGNORED — stamped server-side. */
	published_at?: string;
}

/** `dojo_protocol::PublishResponse`. */
export interface PublishResponse {
	id: string;
	version: number;
	seq: number;
}

/** `dojo_protocol::PulledRule` — the flattened `PublishedRule` plus identity. */
export interface PulledRule extends PublishedRule {
	id: string;
	seq: number;
	status: string;
	version: number;
}

/** `dojo_protocol::PullResponse`. */
export interface PullResponse {
	rules: PulledRule[];
	cursor: number;
}

/** The required string fields of a `PublishedRule` body (validated on publish). */
const REQUIRED_STRING_FIELDS = [
	'content_hash',
	'scope_key',
	'namespace_slug',
	'namespace_name',
	'rule_type',
	'title',
	'content',
	'enforcement'
] as const;

/**
 * Validate + normalize an untrusted publish body into a `PublishedRule`, or
 * return `null` when a required string field is missing/blank (the handler maps
 * that to 400). `impact` / `origin_repo` are optional (null when absent). The
 * client's `published_by` / `published_at` are intentionally NOT read here — the
 * handler stamps them from the authenticated caller.
 */
export function parsePublishedRule(body: Record<string, unknown>): PublishedRule | null {
	for (const f of REQUIRED_STRING_FIELDS) {
		const v = body[f];
		if (typeof v !== 'string' || v.trim() === '') return null;
	}
	const str = (v: unknown): string | null => (typeof v === 'string' && v.trim() !== '' ? v : null);
	return {
		content_hash: body.content_hash as string,
		scope_key: body.scope_key as string,
		namespace_slug: body.namespace_slug as string,
		namespace_name: body.namespace_name as string,
		rule_type: body.rule_type as string,
		title: body.title as string,
		content: body.content as string,
		impact: str(body.impact),
		enforcement: body.enforcement as string,
		origin_repo: str(body.origin_repo)
	};
}

/** A domain error carrying the HTTP status the handler should return. */
export class RulesError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

/**
 * Publish a rule: upsert its namespace (`sensei.namespaces` on `(scope_key,
 * slug)`), then upsert the shared rule (`dojo.shared_rules` on `(namespace_id,
 * content_hash)`). Mirrors `DojoStore::publish`:
 *   • first publish → version 1 (the column default);
 *   • republish of the same (namespace, content_hash) bumps version + refreshes
 *     status/attribution/timestamp.
 *
 * `publishedBy` is the authenticated caller (never the body); `published_at` is
 * stamped `now()` server-side. Returns `PublishResponse { id, version, seq }`.
 *
 * Seq on republish: `seq` is assigned by the `dojo.shared_rules_seq`
 * column default, which fires on INSERT (first publish). PostgREST cannot run
 * `nextval()` inline on the UPDATE (republish) path, so a republish keeps its
 * prior `seq` rather than advancing it — a divergence from dojo-mind, whose
 * store re-runs `seq = nextval(...)` on republish so a re-publish re-surfaces in
 * a puller's delta. Advancing seq on UPDATE needs an in-DB trigger (precedent:
 * `dojo.relay_inbox_seq_bump`), which is a DDL follow-up. First-publish + pull +
 * retract match dojo-mind exactly; content re-sync of an already-known rule is a
 * known follow-up on the daemon side too (`federation/mod.rs` RuleOutcome::Linked).
 */
export async function publishRule(
	db: DojoClient,
	rule: PublishedRule,
	publishedBy: string
): Promise<PublishResponse> {
	// 1. Upsert the namespace in the `sensei` schema (shared_rules.namespace_id
	//    FKs sensei.namespaces, not a dojo table).
	const nsRes = await db
		.schema('sensei')
		.from('namespaces')
		.upsert(
			{ scope_key: rule.scope_key, slug: rule.namespace_slug, name: rule.namespace_name },
			{ onConflict: 'scope_key,slug' }
		)
		.select('id')
		.single();
	if (nsRes.error) throw new RulesError(500, nsRes.error.message);
	const namespaceId = (nsRes.data as { id: string }).id;

	// 2. Upsert the rule. On first insert the DB defaults version=1 and seq via
	//    the sequence; on conflict we bump the version and refresh the snapshot
	//    (attribution + timestamp server-controlled).
	const publishedAt = new Date().toISOString();
	const ruleRes = await db
		.from('shared_rules')
		.upsert(
			{
				namespace_id: namespaceId,
				content_hash: rule.content_hash,
				rule_type: rule.rule_type,
				title: rule.title,
				content: rule.content,
				impact: rule.impact,
				enforcement: rule.enforcement,
				status: 'active',
				origin_repo: rule.origin_repo,
				published_by: publishedBy,
				published_at: publishedAt,
				updated_at: publishedAt
			},
			{ onConflict: 'namespace_id,content_hash' }
		)
		.select('id, version, seq')
		.single();
	if (ruleRes.error) throw new RulesError(500, ruleRes.error.message);
	const row = ruleRes.data as { id: string; version: number; seq: number };
	return { id: row.id, version: row.version, seq: row.seq };
}

/** One `dojo.shared_rules ⋈ sensei.namespaces` row as returned by the pull select. */
interface RuleRow {
	id: string;
	seq: number;
	status: string;
	version: number;
	content_hash: string;
	rule_type: string;
	title: string;
	content: string;
	impact: string | null;
	enforcement: string;
	origin_repo: string | null;
	published_by: string;
	published_at: string;
	// The embedded namespace (PostgREST FK-embed). shared_rules → namespaces is a
	// to-one relationship, so supabase-js returns an object (or, defensively for
	// PostgREST's array-embed shape, a one-element array).
	namespaces: { scope_key: string; slug: string; name: string } | { scope_key: string; slug: string; name: string }[] | null;
}

/** The `select` list for the pull join (shared_rules columns + embedded namespace). */
export const PULL_SELECT =
	'id, seq, status, version, content_hash, rule_type, title, content, impact, enforcement, origin_repo, published_by, published_at, namespaces(scope_key, slug, name)';

/** Normalize PostgREST's embed (object or one-element array) to the single row. */
function embeddedNamespace(ns: RuleRow['namespaces']): { scope_key: string; slug: string; name: string } | null {
	if (!ns) return null;
	return Array.isArray(ns) ? (ns[0] ?? null) : ns;
}

/**
 * Shape a set of joined rows into a `PullResponse` and compute the cursor as
 * `max(seq)` (or `since` when the page is empty) — mirrors
 * `DojoStore::pull_since`. Pure over its input so the wire-shape/cursor math is
 * unit-tested directly.
 */
export function shapePullResponse(rows: RuleRow[], since: number): PullResponse {
	let cursor = since;
	const rules: PulledRule[] = rows.map((r) => {
		if (r.seq > cursor) cursor = r.seq;
		const ns = embeddedNamespace(r.namespaces);
		return {
			id: r.id,
			seq: r.seq,
			status: r.status,
			version: r.version,
			content_hash: r.content_hash,
			scope_key: ns?.scope_key ?? '',
			namespace_slug: ns?.slug ?? '',
			namespace_name: ns?.name ?? '',
			rule_type: r.rule_type,
			title: r.title,
			content: r.content,
			impact: r.impact,
			enforcement: r.enforcement,
			origin_repo: r.origin_repo,
			published_by: r.published_by,
			published_at: r.published_at
		};
	});
	return { rules, cursor };
}

/**
 * Pull all rule deltas with `seq > since`, ordered by `seq` — the query half of
 * `DojoStore::pull_since`. Delegates shaping to `shapePullResponse`.
 */
export async function pullRules(db: DojoClient, since: number): Promise<PullResponse> {
	const { data, error } = await db
		.from('shared_rules')
		.select(PULL_SELECT)
		.gt('seq', since)
		.order('seq', { ascending: true });
	if (error) throw new RulesError(500, error.message);
	return shapePullResponse((data ?? []) as unknown as RuleRow[], since);
}

/**
 * Tombstone a rule (`status → 'tombstoned'`) — the port of `DojoStore::retract`.
 * Only flips a currently non-tombstoned row; returns whether a row changed
 * (`false` → the handler 404s). Mirrors dojo-mind's `WHERE id = $1 AND status <>
 * 'tombstoned'` guard.
 *
 * Seq on retract: as with republish, PostgREST can't advance `seq` inline
 * on this UPDATE; the same in-DB-trigger follow-up applies. The tombstone itself
 * (status flip + audit) matches dojo-mind.
 */
export async function retractRule(db: DojoClient, id: string): Promise<boolean> {
	const { data, error } = await db
		.from('shared_rules')
		.update({ status: 'tombstoned', updated_at: new Date().toISOString() })
		.eq('id', id)
		.neq('status', 'tombstoned')
		.select('id')
		.maybeSingle();
	if (error) throw new RulesError(500, error.message);
	return data != null;
}

/**
 * Append a tenant-scoped audit event for a rules action — the port of
 * dojo-mind's `record_audit` on the publish/retract path, written to the
 * tenant-scoped `dojo.audit_events` (the same non-repudiation trail the console
 * uses; `dojo.audit_log` is member-keyed and not tenant-scoped, so the tenant
 * plane uses audit_events). Fire-and-forget in the handler: a failed audit write
 * must not fail an already-committed publish/retract (mirrors dojo-mind's
 * swallowed audit on the request path), but the error is surfaced to the caller
 * (logged), never silently dropped.
 */
export async function recordRulesAudit(
	db: DojoClient,
	tenantId: string,
	action: 'publish' | 'retract',
	target: string,
	actorId: string,
	detail: Record<string, unknown> = {}
): Promise<void> {
	const { error } = await db.from('audit_events').insert({
		tenant_id: tenantId,
		actor_id: actorId,
		action,
		target,
		detail
	});
	if (error) {
		// Non-fatal (the mutation already committed), but never silent.
		console.error(`rules audit write failed (${action} ${target}): ${error.message}`);
	}
}

// ── Rule-pack resolution (adopt → govern) ────────────────────────────────────
// A namespace that ADOPTS a rule pack inherits the pack's rules. Resolution
// unions, for a set of namespaces, every adopted pack's rules — applying the
// adoption's optional tier override. This is the payoff for the rule_packs DDL:
// an org adopting "Auth boundary guards" makes those rules resolve for its repos
// (and, via the daemon's `/v1/.../rules` federation pull → local memories →
// resolve_rules_raw, flow through the SessionStart/PreCompact push).

/** Enforcement tiers weakest→strongest — mirrors `sensei.enforcement`. */
export const ENFORCEMENT_ORDER = ['advisory', 'recommended', 'required', 'mandatory'] as const;

/** The stronger of two tiers. Unknown tiers rank lowest. Pure. */
export function maxTier(a: string, b: string): string {
	const ia = ENFORCEMENT_ORDER.indexOf(a as (typeof ENFORCEMENT_ORDER)[number]);
	const ib = ENFORCEMENT_ORDER.indexOf(b as (typeof ENFORCEMENT_ORDER)[number]);
	return ib > ia ? b : a;
}

/**
 * The effective tier of a pack rule under an adoption: the rule's own tier,
 * RAISED (never lowered) by the adoption's optional override — so an org can
 * adopt an advisory pack as `required`, but can never weaken a rule below its
 * intrinsic tier (mandatory stays mandatory). Pure.
 */
export function effectivePackRuleTier(ruleTier: string, adoptionOverride: string | null): string {
	return adoptionOverride ? maxTier(ruleTier, adoptionOverride) : ruleTier;
}

/** A pack rule resolved for a namespace — the rule's full shape + its pack's
 *  provenance, with the effective (override-applied) enforcement tier. */
export interface ResolvedPackRule {
	pack_id: string;
	rule_id: string;
	ordinal: number;
	statement: string;
	body: string;
	rationale: string | null;
	enforcement: string; // effective, after the adoption override
	verification: string;
	checker_ref: string | null;
	remediation: string | null;
	skill_ref: string | null;
	applies_to: unknown;
	source: string; // from the pack
	area: string; // from the pack
}

/**
 * Resolve the rules of every pack ADOPTED at any of `namespaceIds`, with each
 * rule's tier raised by the strongest adoption override for its pack among those
 * namespaces. A pack-resolution error is logged and yields `[]` (packs are
 * additive — a hiccup must not break the base rules pull), never silent.
 */
export async function resolveAdoptedPackRules(
	db: DojoClient,
	namespaceIds: string[]
): Promise<ResolvedPackRule[]> {
	if (namespaceIds.length === 0) return [];

	const { data: adoptions, error: aErr } = await db
		.from('rule_pack_adoptions')
		.select('pack_id, enforcement')
		.in('namespace_id', namespaceIds);
	if (aErr) {
		console.error(`rule-pack adoption resolve failed: ${aErr.message}`);
		return [];
	}
	if (!adoptions || adoptions.length === 0) return [];

	// Strongest tier override per pack across the adopting namespaces.
	const overrideByPack = new Map<string, string | null>();
	for (const a of adoptions as Array<{ pack_id: string; enforcement: string | null }>) {
		const prev = overrideByPack.get(a.pack_id);
		if (prev === undefined) overrideByPack.set(a.pack_id, a.enforcement);
		else if (a.enforcement && prev) overrideByPack.set(a.pack_id, maxTier(prev, a.enforcement));
		else overrideByPack.set(a.pack_id, a.enforcement ?? prev ?? null);
	}
	const packIds = [...overrideByPack.keys()];

	const [{ data: packs, error: pErr }, { data: rules, error: rErr }] = await Promise.all([
		db.from('rule_packs').select('id, area, source').in('id', packIds),
		db
			.from('rule_pack_rules')
			.select(
				'id, pack_id, ordinal, statement, body, rationale, enforcement, verification, checker_ref, remediation, skill_ref, applies_to'
			)
			.in('pack_id', packIds)
			.order('ordinal', { ascending: true })
	]);
	if (pErr || rErr) {
		console.error(`rule-pack rule resolve failed: ${(pErr ?? rErr)?.message}`);
		return [];
	}

	const meta = new Map(
		((packs ?? []) as Array<{ id: string; area: string; source: string }>).map((p) => [p.id, p])
	);
	return ((rules ?? []) as Array<Record<string, unknown>>).map((r) => {
		const m = meta.get(r.pack_id as string);
		return {
			pack_id: r.pack_id as string,
			rule_id: r.id as string,
			ordinal: r.ordinal as number,
			statement: r.statement as string,
			body: r.body as string,
			rationale: (r.rationale as string | null) ?? null,
			enforcement: effectivePackRuleTier(
				r.enforcement as string,
				overrideByPack.get(r.pack_id as string) ?? null
			),
			verification: r.verification as string,
			checker_ref: (r.checker_ref as string | null) ?? null,
			remediation: (r.remediation as string | null) ?? null,
			skill_ref: (r.skill_ref as string | null) ?? null,
			applies_to: r.applies_to,
			source: m?.source ?? '',
			area: m?.area ?? ''
		};
	});
}
