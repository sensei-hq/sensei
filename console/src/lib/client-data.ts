// Client-lead console API client + types (R11) over the R8 dojo-mind backend.
//
// Binds to the real client-lead endpoints (crates/dojo-mind/src/api.rs :990-1541),
// all behind the LEAD role-floor (`resolve_lead` → 403 for a non-lead):
//   GET    …/engagements                    → { engagements: Engagement[] }
//   POST   …/engagements                    → { id }
//   PATCH  …/engagements/{id}               → { id }
//   DELETE …/engagements/{id}               → { deleted: true }
//   POST   …/engagements/{id}/bind          → { id, bound: true }
//   GET    …/incidents                      → { incidents: Incident[], open_count }
//   POST   …/incidents                      → { id, severity }
//   PATCH  …/incidents/{id}                 → { id }
//   DELETE …/incidents/{id}                 → { deleted: true }
//   GET    …/audit/artifacts?engagement=&dereferenced=true → AuditArtifact[] (bare array)
//   GET    …/compliance/export?engagement=&format=csv|json → CSV text | { rows }
//
// Row/response types are DERIVED from the Rust store shapes (store.rs
// list_engagements/list_incidents/list_audit_artifacts/export_compliance_rows) and
// the api.rs request bodies (NewEngagement/PatchEngagement/BindProject/NewIncident/
// PatchIncident) — not invented. The shared HTTP primitives (base URL, bearer
// header, tenant encoding, error, JSON parse) are reused from `dojo-api.ts` — the
// same core the triage + admin clients use, per DRY.
//
// Kept in a plain module (base URL read once, fetch injectable) so it imports
// cleanly under SSR/prerender and unit tests without a live backend; the screens
// render off an injected/empty fixture until Jerry wires a real client-lead session.

import { DojoApiError, authHeaders, dojoApiUrl, encodeTenant, parseJson } from './dojo-api';
import type { DojoCallOpts } from './dojo-api';

// Re-export the shared error under the client name so client-lead screens
// `instanceof` against a local symbol without reaching into the shared module.
export { DojoApiError, dojoApiUrl };
export const ClientApiError = DojoApiError;

// ── wire types (mirror the Rust store row shapes) ────────────────────────────

/** An engagement lifecycle state — `dojo.engagement_status` (api.rs `is_engagement_status`). */
export type EngagementStatus = 'active' | 'ended';

/** One engagement row — `store.list_engagements` (store.rs:1139). */
export interface Engagement {
	id: string;
	client: string;
	description: string | null;
	/** jsonb array of `{ project_id, name }` bindings (defaults to `[]`). */
	project_bindings: unknown;
	/** jsonb object of policy overrides (defaults to `{}`). */
	policy_overrides: unknown;
	status: string;
	/** `YYYY-MM-DD`, or null. */
	starts_on: string | null;
	/** `YYYY-MM-DD`, or null. */
	ends_on: string | null;
	/** ISO-8601 `YYYY-MM-DDTHH:MM:SSZ`. */
	created_at: string;
	updated_at: string;
}

/** An incident severity — `dojo.incident_severity` (api.rs `is_incident_severity`). */
export type IncidentSeverity = 'low' | 'medium' | 'high' | 'critical';

/** An incident lifecycle state — `dojo.incident_status` (api.rs `is_incident_status`). */
export type IncidentStatus = 'open' | 'investigating' | 'resolved';

/** One incident row — `store.list_incidents` (store.rs:1330), worst-severity first. */
export interface Incident {
	id: string;
	engagement_id: string | null;
	artifact_id: string | null;
	title: string;
	description: string | null;
	severity: string;
	status: string;
	owner_id: string | null;
	/** ISO-8601, or null. */
	sla_due_at: string | null;
	resolution: string | null;
	/** ISO-8601. */
	opened_at: string;
	/** ISO-8601, or null while open (`resolved_at is null` = open). */
	resolved_at: string | null;
}

/** One artifact-audit row — `store.list_audit_artifacts` (store.rs:1507). */
export interface AuditArtifact {
	id: string;
	seq: number;
	engagement_id: string | null;
	/** artifact kind — `guard | pattern | principle | prompt | skill | agent | persona | …` */
	kind: string;
	title: string;
	/** the universal strip status — `false` is the red-fail (source not dropped). */
	dereferenced: boolean;
	/** the strip verification payload (opaque JSON; what was stripped). */
	attribution: unknown;
	status: string;
	/** ISO-8601, or null. */
	published_at: string | null;
	created_at: string;
}

/** One compliance-export row — `store.export_compliance_rows` (store.rs:1606). The
 * column set is source-ref-free BY CONSTRUCTION on the backend (never a source
 * column); the console mirrors ONLY these columns. */
export interface ComplianceRow {
	artifact_id: string;
	engagement_id: string | null;
	client: string | null;
	kind: string;
	title: string;
	dereferenced: boolean;
	status: string;
	published_at: string | null;
	created_at: string;
}

// ── request bodies (mirror the api.rs Deserialize structs) ───────────────────

/** `POST …/engagements` body — `NewEngagement` (api.rs:1004). `client` required. */
export interface NewEngagementBody {
	client: string;
	description?: string;
	project_bindings?: unknown;
	policy_overrides?: unknown;
	/** `YYYY-MM-DD`. */
	starts_on?: string;
	/** `YYYY-MM-DD`. */
	ends_on?: string;
}

/** `PATCH …/engagements/{id}` body — `PatchEngagement` (api.rs:1059). Close via
 * `status = 'ended'`. An absent field is left unchanged. */
export interface PatchEngagementBody {
	description?: string;
	status?: EngagementStatus;
	starts_on?: string;
	ends_on?: string;
	policy_overrides?: unknown;
	project_bindings?: unknown;
}

/** `POST …/engagements/{id}/bind` body — `BindProject` (api.rs:1123). */
export interface BindProjectBody {
	project_id: string;
	name?: string;
}

/** `POST …/incidents` body — `NewIncident` (api.rs:1216). `title` required;
 * `severity` defaults to `medium` server-side. */
export interface NewIncidentBody {
	title: string;
	description?: string;
	severity?: IncidentSeverity;
	engagement_id?: string;
	artifact_id?: string;
	owner_id?: string;
	/** ISO-8601. */
	sla_due_at?: string;
}

/** `PATCH …/incidents/{id}` body — `PatchIncident` (api.rs:1284). `resolved: true`
 * (or `status: 'resolved'`) closes it; an explicit `open|investigating` reopens. */
export interface PatchIncidentBody {
	severity?: IncidentSeverity;
	status?: IncidentStatus;
	owner_id?: string;
	sla_due_at?: string;
	resolution?: string;
	resolved?: boolean;
}

/** An export format — the `format` query on `…/compliance/export`. */
export type ExportFormat = 'csv' | 'json';

// ── client ───────────────────────────────────────────────────────────────────

/** Build the `/v1/t/{tenant}/…` URL for a client-lead sub-path, tenant-encoded. */
function clientUrl(tenantKey: string, path: string): string {
	return `${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}${path}`;
}

/** A GET returning a JSON envelope, with the bearer header. */
async function getJson<T>(url: string, opts: DojoCallOpts): Promise<T> {
	const f = opts.fetch ?? fetch;
	const res = await f(url, { headers: authHeaders(opts.accessToken) });
	return parseJson<T>(res);
}

/** A body-carrying request (POST/PATCH/DELETE) with the bearer + JSON headers. */
async function sendJson<T>(
	url: string,
	method: 'POST' | 'PATCH' | 'DELETE',
	body: unknown,
	opts: DojoCallOpts
): Promise<T> {
	const f = opts.fetch ?? fetch;
	const res = await f(url, {
		method,
		headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	return parseJson<T>(res);
}

// engagements ─────────────────────────────────────────────────────────────────

/** `GET …/engagements` — the tenant's engagements (client-lead). */
export async function listEngagements(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<Engagement[]> {
	const data = await getJson<{ engagements?: Engagement[] }>(
		clientUrl(tenantKey, '/engagements'),
		opts
	);
	return data.engagements ?? [];
}

/** `POST …/engagements` — register a client engagement (client-lead). */
export async function createEngagement(
	tenantKey: string,
	body: NewEngagementBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string }> {
	return sendJson(clientUrl(tenantKey, '/engagements'), 'POST', body, opts);
}

/** `PATCH …/engagements/{id}` — edit or close (`status: 'ended'`) an engagement. */
export async function updateEngagement(
	tenantKey: string,
	id: string,
	body: PatchEngagementBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string }> {
	return sendJson(
		clientUrl(tenantKey, `/engagements/${encodeURIComponent(id)}`),
		'PATCH',
		body,
		opts
	);
}

/** `POST …/engagements/{id}/bind` — bind a project to the engagement. */
export async function bindEngagementProject(
	tenantKey: string,
	id: string,
	body: BindProjectBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string; bound: boolean }> {
	return sendJson(
		clientUrl(tenantKey, `/engagements/${encodeURIComponent(id)}/bind`),
		'POST',
		body,
		opts
	);
}

/** `DELETE …/engagements/{id}` — hard-delete an engagement (closing is a PATCH). */
export async function deleteEngagement(
	tenantKey: string,
	id: string,
	opts: DojoCallOpts = {}
): Promise<{ deleted: boolean }> {
	return sendJson(
		clientUrl(tenantKey, `/engagements/${encodeURIComponent(id)}`),
		'DELETE',
		undefined,
		opts
	);
}

// incidents ───────────────────────────────────────────────────────────────────

/** The `GET …/incidents` envelope: worst-first rows + the open-count done-gate. */
export interface IncidentList {
	incidents: Incident[];
	/** `count(*)` where `resolved_at is null` (the open-incidents done-gate). */
	open_count: number;
}

/** `GET …/incidents` — the tenant's incidents (worst-first) + `open_count`. */
export async function listIncidents(
	tenantKey: string,
	opts: DojoCallOpts = {}
): Promise<IncidentList> {
	const data = await getJson<{ incidents?: Incident[]; open_count?: number }>(
		clientUrl(tenantKey, '/incidents'),
		opts
	);
	return { incidents: data.incidents ?? [], open_count: data.open_count ?? 0 };
}

/** `POST …/incidents` — open a confidentiality incident (client-lead). */
export async function createIncident(
	tenantKey: string,
	body: NewIncidentBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string; severity: string }> {
	return sendJson(clientUrl(tenantKey, '/incidents'), 'POST', body, opts);
}

/** `PATCH …/incidents/{id}` — update / resolve / reopen an incident. */
export async function updateIncident(
	tenantKey: string,
	id: string,
	body: PatchIncidentBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string }> {
	return sendJson(
		clientUrl(tenantKey, `/incidents/${encodeURIComponent(id)}`),
		'PATCH',
		body,
		opts
	);
}

/** `DELETE …/incidents/{id}` — delete an incident (client-lead). */
export async function deleteIncident(
	tenantKey: string,
	id: string,
	opts: DojoCallOpts = {}
): Promise<{ deleted: boolean }> {
	return sendJson(
		clientUrl(tenantKey, `/incidents/${encodeURIComponent(id)}`),
		'DELETE',
		undefined,
		opts
	);
}

// artifact audit view + compliance export ────────────────────────────────────

/**
 * `GET …/audit/artifacts?engagement={id}[&dereferenced=true]` — the artifact
 * audit view for an engagement. Returns a BARE JSON ARRAY so the console can
 * compute the done-gate directly (`non_dereferenced == count(dereferenced ==
 * false)`). Pass `dereferenced: true` to filter to only stripped rows; the
 * default (all rows) is what surfaces a non-dereferenced row as a red-fail.
 */
export async function listAuditArtifacts(
	tenantKey: string,
	opts: DojoCallOpts & { engagement?: string; dereferenced?: boolean } = {}
): Promise<AuditArtifact[]> {
	const params = new URLSearchParams();
	if (opts.engagement) params.set('engagement', opts.engagement);
	if (opts.dereferenced) params.set('dereferenced', 'true');
	const qs = params.toString();
	const path = qs ? `/audit/artifacts?${qs}` : '/audit/artifacts';
	return getJson<AuditArtifact[]>(clientUrl(tenantKey, path), opts);
}

/** The compliance-export result: JSON rows (`format=json`), or raw CSV text. */
export type ComplianceExport =
	| { format: 'json'; rows: ComplianceRow[] }
	| { format: 'csv'; csv: string };

/**
 * `GET …/compliance/export?engagement={id}&format=csv|json` — trigger the
 * compliance export for an engagement. BLOCKED with a 409 (surfaced as a
 * {@link DojoApiError} carrying the `non-dereferenced` reason) when any in-scope
 * artifact is non-dereferenced — a broken strip must never be certified. On
 * success returns ONLY the source-ref-free subset (never a source ref).
 */
export async function exportCompliance(
	tenantKey: string,
	opts: DojoCallOpts & { engagement?: string; format?: ExportFormat } = {}
): Promise<ComplianceExport> {
	const f = opts.fetch ?? fetch;
	const format: ExportFormat = opts.format ?? 'csv';
	const params = new URLSearchParams();
	if (opts.engagement) params.set('engagement', opts.engagement);
	params.set('format', format);
	const res = await f(clientUrl(tenantKey, `/compliance/export?${params.toString()}`), {
		headers: authHeaders(opts.accessToken)
	});
	if (format === 'json') {
		// parseJson surfaces the 409 blocked-state as a DojoApiError with the reason.
		const data = await parseJson<{ rows?: ComplianceRow[] }>(res);
		return { format: 'json', rows: data.rows ?? [] };
	}
	// CSV path: a non-2xx (e.g. the 409 blocked-state) carries a JSON error body —
	// route it through parseJson so the blocked reason surfaces, not raw CSV/HTML.
	if (!res.ok) {
		await parseJson<unknown>(res);
	}
	return { format: 'csv', csv: await res.text() };
}
