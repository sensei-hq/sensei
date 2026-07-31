// Pure/injectable data logic for the Worker lead incidents endpoint
// (`GET …/incidents`) — the TS port of dojo-mind's `list_incidents` over
// `dojo.incidents`. Read-only this chunk (open/patch are follow-ups). Kept out of
// `+server.ts` so the worst-first ordering + open-count are unit-testable without
// a Worker.
//
// Wire contract: matches the console client `client-data.ts` `listIncidents()` —
// `{ incidents: Incident[], open_count }`, worst-severity first, where
// `open_count = count(resolved_at is null)`.

import type { dojoDb } from './dojo-supabase';
import { resolveDisplayNames, type ResolvedName } from './identity-resolve';
import { resolveEngagementClientNames } from './engagement-client-names';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** A domain error carrying the HTTP status the handler should return. */
export class IncidentsError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

/** One incident row on the wire — the console `Incident`. */
export interface Incident {
	id: string;
	engagement_id: string | null;
	artifact_id: string | null;
	title: string;
	description: string | null;
	severity: string;
	status: string;
	owner_id: string | null;
	sla_due_at: string | null;
	resolution: string | null;
	opened_at: string;
	resolved_at: string | null;
}

/** The `GET …/incidents` envelope. */
export interface IncidentList {
	incidents: Incident[];
	open_count: number;
}

/** The linked artifact on an incident detail (the near-leak artifact it tracks). */
export interface IncidentArtifactRef {
	id: string;
	title: string;
	kind: string;
	status: string;
}

/** The full incident detail (`GET …/incidents/{id}`): the row plus its resolved
 *  client name (engagement → client_name), owner name/email (WS-1), and linked
 *  artifact. */
export interface IncidentDetail extends Incident {
	client_name: string | null;
	owner_name: string | null;
	owner_email: string | null;
	artifact: IncidentArtifactRef | null;
}

const INCIDENT_COLS =
	'id, engagement_id, artifact_id, title, description, severity, status, owner_id, sla_due_at, resolution, opened_at, resolved_at';

/** Severity rank — worst (critical) first. */
const SEVERITY_RANK: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 };

/**
 * Order incidents worst-severity first, then newest-opened; count the open ones
 * (`resolved_at is null`). Pure over its input so the ordering + done-gate are
 * unit-tested directly.
 */
export function shapeIncidents(rows: Incident[]): IncidentList {
	const incidents = [...rows].sort((a, b) => {
		const ra = SEVERITY_RANK[a.severity] ?? 99;
		const rb = SEVERITY_RANK[b.severity] ?? 99;
		if (ra !== rb) return ra - rb;
		return b.opened_at.localeCompare(a.opened_at);
	});
	const open_count = incidents.reduce((n, i) => n + (i.resolved_at == null ? 1 : 0), 0);
	return { incidents, open_count };
}

/**
 * List the tenant's incidents, worst-first, with the open-count done-gate. The
 * port of dojo-mind's `list_incidents`.
 */
export async function listIncidents(db: DojoClient, tenantId: string): Promise<IncidentList> {
	const { data, error } = await db
		.from('incidents')
		.select(INCIDENT_COLS)
		.eq('tenant_id', tenantId)
		.order('opened_at', { ascending: false });
	if (error) throw new IncidentsError(500, error.message);
	return shapeIncidents((data ?? []) as unknown as Incident[]);
}

const INCIDENT_ARTIFACT_COLS = 'id, title, kind, status';

/** Fetch the linked artifact for an incident detail (or null when unlinked).
 *  Fail-closed on a query error. */
async function fetchIncidentArtifact(
	db: DojoClient,
	tenantId: string,
	artifactId: string | null
): Promise<IncidentArtifactRef | null> {
	if (!artifactId) return null;
	const { data, error } = await db
		.from('artifacts')
		.select(INCIDENT_ARTIFACT_COLS)
		.eq('tenant_id', tenantId)
		.eq('id', artifactId)
		.maybeSingle();
	if (error) throw new IncidentsError(500, error.message);
	return data ? (data as unknown as IncidentArtifactRef) : null;
}

/**
 * Fetch one incident (tenant-scoped) with its resolved client name (engagement →
 * `client_name`), owner name/email (WS-1 `resolveDisplayNames`), and linked
 * artifact — the `GET …/incidents/{id}` detail pane read. Throws
 * IncidentsError(404) when the incident doesn't exist; fails closed on any
 * resolve error (never a fabricated name/artifact).
 */
export async function getIncidentDetail(
	db: DojoClient,
	tenantId: string,
	id: string
): Promise<IncidentDetail> {
	const { data, error } = await db
		.from('incidents')
		.select(INCIDENT_COLS)
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.maybeSingle();
	if (error) throw new IncidentsError(500, error.message);
	if (!data) throw new IncidentsError(404, 'no such incident');
	const incident = data as unknown as Incident;

	const [clientNames, ownerNames, artifact] = await Promise.all([
		incident.engagement_id
			? resolveEngagementClientNames(db, tenantId, [incident.engagement_id])
			: Promise.resolve(new Map<string, string>()),
		incident.owner_id
			? resolveDisplayNames(db, tenantId, [incident.owner_id])
			: Promise.resolve(new Map<string, ResolvedName>()),
		fetchIncidentArtifact(db, tenantId, incident.artifact_id)
	]);

	const owner = incident.owner_id ? ownerNames.get(incident.owner_id) : undefined;
	return {
		...incident,
		client_name: incident.engagement_id ? (clientNames.get(incident.engagement_id) ?? null) : null,
		owner_name: owner?.display_name ?? null,
		owner_email: owner?.email ?? null,
		artifact
	};
}

// ════════════════════════════════════════════════════════════════════════════
// Writes — the port of dojo-mind's incident mutations (open · patch/resolve/reopen
// · delete), lead-gated. Each validates its untrusted body (throwing
// IncidentsError 400) and returns the shape the console client `client-data.ts`
// expects. Pure over `db`; the handler records the audit row after the write.
// ════════════════════════════════════════════════════════════════════════════

const SEVERITIES = ['low', 'medium', 'high', 'critical'] as const;
const STATUSES = ['open', 'investigating', 'resolved'] as const;

function isSeverity(v: unknown): boolean {
	return typeof v === 'string' && (SEVERITIES as readonly string[]).includes(v);
}
function isStatus(v: unknown): boolean {
	return typeof v === 'string' && (STATUSES as readonly string[]).includes(v);
}

/** A validated `POST …/incidents` body — the port of dojo-mind's `NewIncident`.
 *  `title` required; `severity` defaults to `medium` server-side. */
export interface NewIncidentInput {
	title: string;
	description?: string;
	severity: string;
	engagement_id?: string;
	artifact_id?: string;
	owner_id?: string;
	sla_due_at?: string;
}

/** Validate a `POST …/incidents` body into a {@link NewIncidentInput}, or throw
 *  IncidentsError(400). `severity` (when present) must be an
 *  `dojo.incident_severity`; it defaults to `medium`. */
export function parseNewIncident(body: Record<string, unknown>): NewIncidentInput {
	const title = typeof body.title === 'string' ? body.title.trim() : '';
	if (!title) throw new IncidentsError(400, 'title is required');
	if (body.severity !== undefined && !isSeverity(body.severity)) {
		throw new IncidentsError(400, 'severity must be low, medium, high, or critical');
	}
	const out: NewIncidentInput = {
		title,
		severity: isSeverity(body.severity) ? (body.severity as string) : 'medium'
	};
	if (typeof body.description === 'string') out.description = body.description;
	if (typeof body.engagement_id === 'string') out.engagement_id = body.engagement_id;
	if (typeof body.artifact_id === 'string') out.artifact_id = body.artifact_id;
	if (typeof body.owner_id === 'string') out.owner_id = body.owner_id;
	if (typeof body.sla_due_at === 'string') out.sla_due_at = body.sla_due_at;
	return out;
}

/**
 * Open a confidentiality incident (`POST …/incidents`) — the port of dojo-mind's
 * `open_incident`. Returns `{ id, severity }` (the shape `client-data.ts
 * createIncident()` unwraps).
 */
export async function createIncident(
	db: DojoClient,
	tenantId: string,
	input: NewIncidentInput
): Promise<{ id: string; severity: string }> {
	const { data, error } = await db
		.from('incidents')
		.insert({
			tenant_id: tenantId,
			title: input.title,
			description: input.description ?? null,
			severity: input.severity,
			engagement_id: input.engagement_id ?? null,
			artifact_id: input.artifact_id ?? null,
			owner_id: input.owner_id ?? null,
			sla_due_at: input.sla_due_at ?? null
		})
		.select('id, severity')
		.single();
	if (error) throw new IncidentsError(500, error.message);
	return data as { id: string; severity: string };
}

/** A validated `PATCH …/incidents/{id}` body — the port of dojo-mind's
 *  `PatchIncident`. Resolving stamps `status='resolved'` + `resolved_at=now`;
 *  reopening (`open|investigating`) clears `resolved_at`. */
export interface PatchIncidentInput {
	severity?: string;
	status?: string;
	owner_id?: string;
	sla_due_at?: string;
	resolution?: string;
	/** the derived close flag — set when resolving (status='resolved' or
	 *  resolved:true), so the store stamps/clears resolved_at. */
	resolve?: boolean;
	/** an explicit reopen (status is open|investigating) — clears resolved_at. */
	reopen?: boolean;
}

/**
 * Validate a `PATCH …/incidents/{id}` body into a {@link PatchIncidentInput}, or
 * throw IncidentsError(400). `resolved: true` OR `status: 'resolved'` closes it;
 * an explicit `open|investigating` reopens it (clearing `resolved_at`). At least
 * one field must be present.
 */
export function parsePatchIncident(body: Record<string, unknown>): PatchIncidentInput {
	const out: PatchIncidentInput = {};
	if (body.severity !== undefined) {
		if (!isSeverity(body.severity)) {
			throw new IncidentsError(400, 'severity must be low, medium, high, or critical');
		}
		out.severity = body.severity as string;
	}
	if (body.status !== undefined) {
		if (!isStatus(body.status)) {
			throw new IncidentsError(400, 'status must be open, investigating, or resolved');
		}
		out.status = body.status as string;
		if (body.status === 'resolved') out.resolve = true;
		else out.reopen = true;
	}
	if (body.resolved === true) {
		out.resolve = true;
		if (out.status === undefined) out.status = 'resolved';
	}
	if (typeof body.owner_id === 'string') out.owner_id = body.owner_id;
	if (typeof body.sla_due_at === 'string') out.sla_due_at = body.sla_due_at;
	if (typeof body.resolution === 'string') out.resolution = body.resolution;
	if (
		out.severity === undefined &&
		out.status === undefined &&
		out.owner_id === undefined &&
		out.sla_due_at === undefined &&
		out.resolution === undefined &&
		!out.resolve
	) {
		throw new IncidentsError(400, 'no updatable fields in the request');
	}
	return out;
}

/**
 * Patch / resolve / reopen an incident (`PATCH …/incidents/{id}`) — the port of
 * dojo-mind's `patch_incident`. Resolving stamps `resolved_at`; reopening clears
 * it. Returns `{ id }`; a 404 when no tenant incident matches.
 */
export async function updateIncident(
	db: DojoClient,
	tenantId: string,
	id: string,
	input: PatchIncidentInput
): Promise<{ id: string }> {
	const row: Record<string, unknown> = { updated_at: new Date().toISOString() };
	if (input.severity !== undefined) row.severity = input.severity;
	if (input.status !== undefined) row.status = input.status;
	if (input.owner_id !== undefined) row.owner_id = input.owner_id;
	if (input.sla_due_at !== undefined) row.sla_due_at = input.sla_due_at;
	if (input.resolution !== undefined) row.resolution = input.resolution;
	if (input.resolve) row.resolved_at = new Date().toISOString();
	else if (input.reopen) row.resolved_at = null;
	const { data, error } = await db
		.from('incidents')
		.update(row)
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id')
		.maybeSingle();
	if (error) throw new IncidentsError(500, error.message);
	if (!data) throw new IncidentsError(404, 'no such incident');
	return data as { id: string };
}

/**
 * Delete an incident by id (`DELETE …/incidents/{id}`) — returns `true` when a
 * row was removed, `false` when none matched (the handler maps false → 404).
 */
export async function deleteIncident(db: DojoClient, tenantId: string, id: string): Promise<boolean> {
	const { data, error } = await db
		.from('incidents')
		.delete()
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id');
	if (error) throw new IncidentsError(500, error.message);
	return (data?.length ?? 0) > 0;
}
