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
// A shared rule belongs to one tenant (#236): unique on (tenant_id,
// namespace_id, content_hash), read by its members and written by its
// maintainers under the policies in database/policies/dojo/shared_rules.sql.
// The caller is resolved by `resolveTenantCaller`; the tenant is also recorded
// on the `dojo.audit_events` row.

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
 * Register a rule's namespace (`sensei.namespaces` on `(scope_key, slug)`) and
 * return its id. The namespace registry is shared across tenants and no session
 * writes it, so this is the system's (#236 step 4) — `publishRule` then files the
 * rule under the id as the person.
 */
export async function upsertRuleNamespace(db: DojoClient, rule: PublishedRule): Promise<string> {
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
	return (nsRes.data as { id: string }).id;
}

/**
 * Publish a rule: upsert the shared rule (`dojo.shared_rules` on `(tenant_id,
 * namespace_id, content_hash)`) under the namespace `upsertRuleNamespace`
 * registered. Mirrors `DojoStore::publish`:
 *   • first publish → version 1 (the column default);
 *   • republish of the same (namespace, content_hash) bumps version + refreshes
 *     status/attribution/timestamp.
 *
 * `publishedBy` is the authenticated caller (never the body); `published_at` is
 * stamped `now()` server-side. Returns `PublishResponse { id, version, seq }`.
 *
 * Seq on republish: an upsert that updates an existing row advances `seq`
 * through the `dojo.shared_rules_seq_bump` trigger, so a republish re-surfaces
 * in a puller's delta exactly as dojo-mind's inline `nextval` did.
 * Content re-sync of an already-known rule is a known follow-up on the daemon
 * side (`federation/mod.rs` RuleOutcome::Linked).
 */
export async function publishRule(
	db: DojoClient,
	tenantId: string,
	rule: PublishedRule,
	publishedBy: string,
	namespaceId: string
): Promise<PublishResponse> {
	// On first insert the DB defaults version=1 and seq via
	//    the sequence; on conflict we bump the version and refresh the snapshot
	//    (attribution + timestamp server-controlled).
	const publishedAt = new Date().toISOString();
	const ruleRes = await db
		.from('shared_rules')
		.upsert(
			{
				// The rule is the publishing TENANT's (#236): stamped with it and
				// unique within it, so no tenant's publish lands on another's row.
				tenant_id: tenantId,
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
			{ onConflict: 'tenant_id,namespace_id,content_hash' }
		)
		.select('id, version, seq')
		.single();
	if (ruleRes.error) throw new RulesError(500, ruleRes.error.message);
	const row = ruleRes.data as { id: string; version: number; seq: number };
	return { id: row.id, version: row.version, seq: row.seq };
}

/** One row of `dojo.tenant_shared_rules` — a shared rule with its namespace. */
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
	scope_key: string;
	namespace_slug: string;
	namespace_name: string;
}

/** The legacy select's row: the rule with its namespace EMBEDDED (PostgREST
 *  FK-embed — an object, or defensively a one-element array). */
type EmbeddedRuleRow = Omit<RuleRow, 'scope_key' | 'namespace_slug' | 'namespace_name'> & {
	namespaces: { scope_key: string; slug: string; name: string } | { scope_key: string; slug: string; name: string }[] | null;
};

/** The `select` list for the legacy pull join (shared_rules columns + embedded namespace). */
export const PULL_SELECT =
	'id, seq, status, version, content_hash, rule_type, title, content, impact, enforcement, origin_repo, published_by, published_at, namespaces(scope_key, slug, name)';

/** Normalize PostgREST's embed (object or one-element array) to the single row. */
function embeddedNamespace(ns: EmbeddedRuleRow['namespaces']): { scope_key: string; slug: string; name: string } | null {
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
		return {
			id: r.id,
			seq: r.seq,
			status: r.status,
			version: r.version,
			content_hash: r.content_hash,
			scope_key: r.scope_key,
			namespace_slug: r.namespace_slug,
			namespace_name: r.namespace_name,
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
 * Pull all rule deltas with `seq > since`, in `seq` order, AS THE PERSON —
 * `dojo.tenant_shared_rules`, which carries each rule's namespace (a session
 * cannot read `sensei.namespaces` to embed it). The query half of
 * `DojoStore::pull_since`; shaping is `shapePullResponse`. A refusal (a
 * non-member, 42501) or any error is a RulesError(500), never an empty page.
 */
export async function pullRules(
	db: DojoClient,
	tenantId: string,
	since: number
): Promise<PullResponse> {
	const { data, error } = await db.rpc('tenant_shared_rules', { p_tenant: tenantId, p_since: since });
	if (error) throw new RulesError(500, error.message);
	return shapePullResponse((data ?? []) as RuleRow[], since);
}

/**
 * The same pull for the legacy device token (the service role), during the one
 * release window of #236 step 4 — removed with the device token in 4e. The
 * route's membership check already scoped the caller to `tenantId`.
 */
export async function pullRulesAsSystem(
	db: DojoClient,
	tenantId: string,
	since: number
): Promise<PullResponse> {
	const { data, error } = await db
		.from('shared_rules')
		.select(PULL_SELECT)
		.eq('tenant_id', tenantId)
		.gt('seq', since)
		.order('seq', { ascending: true });
	if (error) throw new RulesError(500, error.message);
	const rows = ((data ?? []) as unknown as EmbeddedRuleRow[]).map(({ namespaces, ...r }) => {
		const ns = embeddedNamespace(namespaces);
		return { ...r, scope_key: ns?.scope_key ?? '', namespace_slug: ns?.slug ?? '', namespace_name: ns?.name ?? '' };
	});
	return shapePullResponse(rows, since);
}

/**
 * Tombstone a rule (`status → 'tombstoned'`) — the port of `DojoStore::retract`.
 * Only flips a currently non-tombstoned row; returns whether a row changed
 * (`false` → the handler 404s). Mirrors dojo-mind's `WHERE id = $1 AND status <>
 * 'tombstoned'` guard.
 *
 * Seq on retract: the `dojo.shared_rules_seq_bump` trigger advances `seq` on
 * this UPDATE, so a puller already past the rule's insert seq still learns of
 * the tombstone. The status flip + audit matches dojo-mind.
 */
export async function retractRule(db: DojoClient, tenantId: string, id: string): Promise<boolean> {
	const { data, error } = await db
		.from('shared_rules')
		.update({ status: 'tombstoned', updated_at: new Date().toISOString() })
		.eq('id', id)
		.eq('tenant_id', tenantId)
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
// A namespace that ADOPTS a rule pack inherits the pack's rules. The daemon asks,
// for its folder's namespaces, for the rules of every pack adopted there — and a
// member gets those adopted at the namespace their TENANT owns
// (`dojo.tenant_pack_rules`, decision 2026-10-12), each at the adoption's scope
// with its never-weaken tier override. They flow through the SessionStart /
// PreCompact push.

/** Parse the `?ns=organization:acme,stack:react` query into (scope_key, slug)
 *  pairs — how the daemon names its repo's namespaces cross-DB (the stable key,
 *  not the dojo-side uuid). Malformed segments are dropped. Pure. */
export function parseNamespacePairs(raw: string | null): Array<{ scope_key: string; slug: string }> {
	return (raw ?? '')
		.split(',')
		.map((s) => s.trim())
		.filter(Boolean)
		.map((s) => {
			const i = s.indexOf(':');
			return i < 0
				? { scope_key: '', slug: '' } // no colon → dropped by the filter
				: { scope_key: s.slice(0, i), slug: s.slice(i + 1) };
		})
		.filter((p) => p.scope_key && p.slug);
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
	source: string; // the pack's attribution
	area: string; // from the pack (the category — provenance only)
	/** The GOVERNANCE scope the pack was ADOPTED at (the adopting namespace's
	 *  scope_key: general/organization/project/…). This — not `area` — is the
	 *  scope the rule governs at, so a federated pack rule groups on the same
	 *  ladder axis as a memory (parity with the daemon's LOCAL resolver). */
	scope_key: string;
}

/** One row of `dojo.tenant_pack_rules`. */
type TenantPackRuleRow = Omit<ResolvedPackRule, 'source'> & { attribution: string };

/**
 * The rules of the packs adopted at the namespace the caller's tenant owns,
 * among the folder's `pairs` — `dojo.tenant_pack_rules`, as the person (members
 * only; the tier override, the adoption scope and the ownership are the
 * function's). No pairs asks nothing and is no rules. Any error — a refusal
 * included — is a RulesError(500): never an empty list that reads as "no packs".
 */
export async function resolveTenantPackRules(
	db: DojoClient,
	tenantId: string,
	pairs: Array<{ scope_key: string; slug: string }>
): Promise<ResolvedPackRule[]> {
	if (pairs.length === 0) return [];
	const { data, error } = await db.rpc('tenant_pack_rules', {
		p_tenant: tenantId,
		p_namespaces: pairs.map((p) => `${p.scope_key}:${p.slug}`)
	});
	if (error) throw new RulesError(500, error.message);
	return ((data ?? []) as TenantPackRuleRow[]).map(({ attribution, ...r }) => ({ ...r, source: attribution }));
}
