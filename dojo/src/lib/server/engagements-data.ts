// Pure/injectable write logic for the Worker lead engagements endpoints
// (`PATCH …/engagements/{id}` · `DELETE …/engagements/{id}` ·
// `POST …/engagements/{id}/bind`) — the TS port of dojo-mind's `patch_engagement`
// / `delete_engagement` / `bind_project` over `dojo.engagements`, lead-gated. The
// list + create verbs are inline in `engagements/+server.ts` (the reference GET/
// POST); the detail/bind verbs live here so the patch shaping + the bind
// merge-into-jsonb are unit-testable without a Worker.
//
// Wire contract: matches the console client `client-data.ts` engagement fns —
// `PATCH` / `POST bind` return `{ id }` / `{ id, bound: true }`; `DELETE` returns
// `{ deleted: true }`. Close is a PATCH with `status: 'ended'`.

import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** A domain error carrying the HTTP status the handler should return. */
export class EngagementsError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const STATUSES = ['active', 'ended'] as const;
function isStatus(v: unknown): boolean {
	return typeof v === 'string' && (STATUSES as readonly string[]).includes(v);
}

/** A validated `PATCH …/engagements/{id}` body — the port of dojo-mind's
 *  `PatchEngagement` (every field optional; an absent field is left unchanged).
 *  Close via `status: 'ended'`. */
export interface PatchEngagementInput {
	description?: string;
	status?: string;
	starts_on?: string;
	ends_on?: string;
	policy_overrides?: unknown;
	project_bindings?: unknown;
}

/**
 * Validate a `PATCH …/engagements/{id}` body into a {@link PatchEngagementInput},
 * or throw EngagementsError(400). `status` (when present) must be a
 * `dojo.engagement_status`. At least one updatable field must be present.
 */
export function parsePatchEngagement(body: Record<string, unknown>): PatchEngagementInput {
	const out: PatchEngagementInput = {};
	if (body.status !== undefined) {
		if (!isStatus(body.status)) throw new EngagementsError(400, 'status must be active or ended');
		out.status = body.status as string;
	}
	if (typeof body.description === 'string') out.description = body.description;
	if (typeof body.starts_on === 'string') out.starts_on = body.starts_on;
	if (typeof body.ends_on === 'string') out.ends_on = body.ends_on;
	if (body.policy_overrides !== undefined) out.policy_overrides = body.policy_overrides;
	if (body.project_bindings !== undefined) out.project_bindings = body.project_bindings;
	if (Object.keys(out).length === 0) {
		throw new EngagementsError(400, 'no updatable fields in the request');
	}
	return out;
}

/**
 * Patch / close (`status: 'ended'`) an engagement (`PATCH …/engagements/{id}`) —
 * the port of dojo-mind's `patch_engagement`. Returns `{ id }`; a 404 when no
 * tenant engagement matches.
 */
export async function updateEngagement(
	db: DojoClient,
	tenantId: string,
	id: string,
	input: PatchEngagementInput
): Promise<{ id: string }> {
	const row: Record<string, unknown> = { updated_at: new Date().toISOString() };
	if (input.description !== undefined) row.description = input.description;
	if (input.status !== undefined) row.status = input.status;
	if (input.starts_on !== undefined) row.starts_on = input.starts_on;
	if (input.ends_on !== undefined) row.ends_on = input.ends_on;
	if (input.policy_overrides !== undefined) row.policy_overrides = input.policy_overrides;
	if (input.project_bindings !== undefined) row.project_bindings = input.project_bindings;
	const { data, error } = await db
		.from('engagements')
		.update(row)
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id')
		.maybeSingle();
	if (error) throw new EngagementsError(500, error.message);
	if (!data) throw new EngagementsError(404, 'no such engagement');
	return data as { id: string };
}

/**
 * Delete an engagement by id (`DELETE …/engagements/{id}`) — returns `true` when
 * a row was removed, `false` when none matched (the handler maps false → 404).
 * Closing (retain-for-audit) is a PATCH `status: 'ended'`; this is a hard delete.
 */
export async function deleteEngagement(
	db: DojoClient,
	tenantId: string,
	id: string
): Promise<boolean> {
	const { data, error } = await db
		.from('engagements')
		.delete()
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id');
	if (error) throw new EngagementsError(500, error.message);
	return (data?.length ?? 0) > 0;
}

/** A validated `POST …/engagements/{id}/bind` body — the port of dojo-mind's
 *  `BindProject`. `project_id` required; `name` optional. */
export interface BindProjectInput {
	project_id: string;
	name?: string;
}

/** One `{ project_id, name }` binding off the jsonb array. */
interface ProjectBinding {
	project_id?: unknown;
	name?: unknown;
}

/** Validate a `POST …/engagements/{id}/bind` body into a {@link BindProjectInput}. */
export function parseBindProject(body: Record<string, unknown>): BindProjectInput {
	const projectId = typeof body.project_id === 'string' ? body.project_id.trim() : '';
	if (!projectId) throw new EngagementsError(400, 'project_id is required');
	const out: BindProjectInput = { project_id: projectId };
	if (typeof body.name === 'string') out.name = body.name;
	return out;
}

/**
 * Merge a `{ project_id, name }` binding into an existing `project_bindings`
 * jsonb array — idempotent on `project_id` (a re-bind updates the name rather
 * than duplicating the row). Pure over its input.
 */
export function mergeBinding(existing: unknown, binding: BindProjectInput): ProjectBinding[] {
	const rows: ProjectBinding[] = Array.isArray(existing) ? [...(existing as ProjectBinding[])] : [];
	const idx = rows.findIndex((b) => b?.project_id === binding.project_id);
	const next: ProjectBinding = { project_id: binding.project_id };
	if (binding.name !== undefined) next.name = binding.name;
	if (idx >= 0) rows[idx] = { ...rows[idx], ...next };
	else rows.push(next);
	return rows;
}

/**
 * Bind a project to an engagement (`POST …/engagements/{id}/bind`) — the port of
 * dojo-mind's `bind_project`. Reads the engagement's current `project_bindings`,
 * merges the binding (idempotent on project_id), and writes it back. Returns
 * `{ id, bound: true }` (the shape `client-data.ts bindEngagementProject()`
 * unwraps). A 404 is thrown when no tenant engagement matches.
 */
export async function bindEngagementProject(
	db: DojoClient,
	tenantId: string,
	id: string,
	input: BindProjectInput
): Promise<{ id: string; bound: boolean }> {
	// Read the current bindings (tenant-scoped) so a re-bind merges, not clobbers.
	const { data: current, error: findErr } = await db
		.from('engagements')
		.select('project_bindings')
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.maybeSingle();
	if (findErr) throw new EngagementsError(500, findErr.message);
	if (!current) throw new EngagementsError(404, 'no such engagement');

	const merged = mergeBinding((current as { project_bindings: unknown }).project_bindings, input);
	const { data, error } = await db
		.from('engagements')
		.update({ project_bindings: merged, updated_at: new Date().toISOString() })
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id')
		.maybeSingle();
	if (error) throw new EngagementsError(500, error.message);
	if (!data) throw new EngagementsError(404, 'no such engagement');
	return { id: (data as { id: string }).id, bound: true };
}
