// Console read for the maintainer Knowledge library (`GET …/knowledge`) — the
// tenant's published `dojo.artifacts`, partitioned for the screen:
//   • catalog — the shareable extensions: kind ∈ {skill, agent, prompt}
//   • active  — published knowledge (principle/pattern/guard) still in use
//   • pending — published knowledge unused past the prune window
//               (retention_days from dojo.policies; no adoption + older than window)
// Read-only for v1 (edits happen via triage/authoring). This replaces the
// `knowledgeFor(slug)` fixture that rendered a fabricated library for real orgs
// (2026-07-29 fallback audit) — so it fails CLOSED: a query error throws, the
// loader surfaces an error state, never a fixture.
import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** A domain error carrying the HTTP status the handler should return. */
export class KnowledgeError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

/** One published artifact row, projected for the library. */
export interface KnowledgeArtifact {
	id: string;
	kind: string;
	title: string;
	/** `dojo.artifacts.scope` jsonb ({company|team|project|stack}), verbatim. */
	scope: unknown;
	adopted_count: number;
	created_at: string;
}

/** The `GET …/knowledge` envelope — raw wire rows partitioned; the client maps
 *  each to the kit shape (kind→kanji, scope→label, age). */
export interface KnowledgeLibrary {
	/** The prune window (`dojo.policies.retention_days`), or null when unset. */
	retention_days: number | null;
	active: KnowledgeArtifact[];
	pending: KnowledgeArtifact[];
	catalog: KnowledgeArtifact[];
}

/** Extension kinds — the shareable Catalog (skills/agents/prompts). */
const CATALOG_KINDS = new Set(['skill', 'agent', 'prompt']);

const KNOWLEDGE_COLS = 'id, kind, title, scope, adopted_count, created_at';

/** Whole days between an ISO timestamp and `now` (negative clamped to 0). */
function ageInDays(iso: string, now: Date): number {
	const ms = now.getTime() - new Date(iso).getTime();
	return ms <= 0 ? 0 : Math.floor(ms / 86_400_000);
}

/**
 * Partition published artifacts into catalog / active / pending (pure). An
 * extension kind → catalog; otherwise a knowledge artifact is `pending` when a
 * prune window is set AND it has no adoption AND it's older than the window,
 * else `active`. `retentionDays === null` means no pruning → nothing is pending.
 */
export function partitionKnowledge(
	rows: KnowledgeArtifact[],
	retentionDays: number | null,
	now: Date
): Pick<KnowledgeLibrary, 'active' | 'pending' | 'catalog'> {
	const active: KnowledgeArtifact[] = [];
	const pending: KnowledgeArtifact[] = [];
	const catalog: KnowledgeArtifact[] = [];
	for (const r of rows) {
		if (CATALOG_KINDS.has(r.kind)) {
			catalog.push(r);
			continue;
		}
		const unusedPastWindow =
			retentionDays !== null && r.adopted_count === 0 && ageInDays(r.created_at, now) > retentionDays;
		(unusedPastWindow ? pending : active).push(r);
	}
	return { active, pending, catalog };
}

/** The tightest prune window across the tenant's policies (min non-null
 *  `retention_days`), or null when none set. Pure. */
export function tightestRetention(rows: { retention_days: number | null }[]): number | null {
	let min: number | null = null;
	for (const r of rows) {
		if (r.retention_days == null) continue;
		if (min === null || r.retention_days < min) min = r.retention_days;
	}
	return min;
}

/**
 * Read the tenant's published-artifact library + prune window, partitioned. Two
 * tenant-scoped reads (artifacts, policies). Fails closed on either error
 * (KnowledgeError 500) — never a fabricated/partial library.
 */
export async function getKnowledgeLibrary(
	db: DojoClient,
	tenantId: string,
	now: Date = new Date()
): Promise<KnowledgeLibrary> {
	const [artifactsRes, policiesRes] = await Promise.all([
		db
			.from('artifacts')
			.select(KNOWLEDGE_COLS)
			.eq('tenant_id', tenantId)
			.eq('status', 'published')
			.order('created_at', { ascending: false }),
		db.from('policies').select('retention_days').eq('tenant_id', tenantId)
	]);
	if (artifactsRes.error) throw new KnowledgeError(500, artifactsRes.error.message);
	if (policiesRes.error) throw new KnowledgeError(500, policiesRes.error.message);
	const retention_days = tightestRetention(
		(policiesRes.data ?? []) as { retention_days: number | null }[]
	);
	const parts = partitionKnowledge(
		(artifactsRes.data ?? []) as unknown as KnowledgeArtifact[],
		retention_days,
		now
	);
	return { retention_days, ...parts };
}
