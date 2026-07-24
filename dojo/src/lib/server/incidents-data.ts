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
