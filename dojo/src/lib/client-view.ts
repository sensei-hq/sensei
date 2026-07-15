// Pure presentation helpers for the lead console screens (R11).
//
// Side-effect-free (data in => display value out) so they unit-test without a DOM
// or a live backend, and so the screens stay declarative. Colour is a token
// utility CLASS name (text-accent / text-success / text-danger / …) per the
// console design system — never a raw hex/oklch. Timestamp helpers are reused
// from `admin-view` (relativeAge / clockTime / shortId) — not re-implemented.

import type { Engagement } from './client-data';

// ── engagement status => label + tone ────────────────────────────────────────

const ENGAGEMENT_STATUS_LABELS: Record<string, string> = {
	active: 'Active',
	ended: 'Ended'
};

/** Human label for an engagement status; falls back to a titled raw value. */
export function engagementStatusLabel(status: string): string {
	return ENGAGEMENT_STATUS_LABELS[status] ?? titleize(status);
}

/** Token tone for an engagement status: active reads success, ended reads muted. */
export function engagementStatusToneClass(status: string): string {
	return status === 'active' ? 'text-success' : 'text-ink-mute';
}

// ── project bindings (opaque jsonb array) => usable rows ─────────────────────

/** One `{ project_id, name }` binding on an engagement. */
export interface ProjectBinding {
	project_id: string;
	name: string | null;
}

/**
 * Normalise an engagement's `project_bindings` jsonb (an array of
 * `{ project_id, name }`) into a typed list. Anything that isn't a well-formed
 * binding object is skipped rather than thrown — the wire value is opaque.
 */
export function projectBindings(engagement: Pick<Engagement, 'project_bindings'>): ProjectBinding[] {
	const raw = engagement.project_bindings;
	if (!Array.isArray(raw)) return [];
	const out: ProjectBinding[] = [];
	for (const item of raw) {
		if (item && typeof item === 'object' && 'project_id' in item) {
			const pid = (item as { project_id: unknown }).project_id;
			if (typeof pid !== 'string' || pid.length === 0) continue;
			const name = 'name' in item ? (item as { name: unknown }).name : null;
			out.push({ project_id: pid, name: typeof name === 'string' ? name : null });
		}
	}
	return out;
}

/** A compact one-line summary of an engagement's project bindings (mockup scopes). */
export function bindingsSummary(engagement: Pick<Engagement, 'project_bindings'>): string {
	const b = projectBindings(engagement);
	if (b.length === 0) return '—';
	return b.map((x) => x.name ?? x.project_id).join(' · ');
}

// ── incident severity => rank + label + tone ─────────────────────────────────

/** Explicit worst-first rank for a severity (mirrors the store's ORDER BY rank). */
const SEVERITY_RANK: Record<string, number> = {
	critical: 4,
	high: 3,
	medium: 2,
	low: 1
};

/** Numeric worst-first rank for a severity (unknown => 0). */
export function severityRank(severity: string): number {
	return SEVERITY_RANK[severity] ?? 0;
}

const SEVERITY_LABELS: Record<string, string> = {
	low: 'Low',
	medium: 'Medium',
	high: 'High',
	critical: 'Critical'
};

/** Human label for an incident severity. */
export function severityLabel(severity: string): string {
	return SEVERITY_LABELS[severity] ?? titleize(severity);
}

/** Token tone for a severity: critical/high read danger, medium accent, low muted. */
export function severityToneClass(severity: string): string {
	switch (severity) {
		case 'critical':
		case 'high':
			return 'text-danger';
		case 'medium':
			return 'text-accent';
		case 'low':
			return 'text-ink-soft';
		default:
			return 'text-ink-mute';
	}
}

/** The four selectable severities, worst-first (for a severity picker). */
export const INCIDENT_SEVERITIES: readonly string[] = ['critical', 'high', 'medium', 'low'] as const;

// ── incident status => label + tone ──────────────────────────────────────────

const INCIDENT_STATUS_LABELS: Record<string, string> = {
	open: 'Open',
	investigating: 'Investigating',
	resolved: 'Resolved'
};

/** Human label for an incident status. */
export function incidentStatusLabel(status: string): string {
	return INCIDENT_STATUS_LABELS[status] ?? titleize(status);
}

/** Token tone for an incident status: resolved reads success, open danger, else accent. */
export function incidentStatusToneClass(status: string): string {
	switch (status) {
		case 'resolved':
			return 'text-success';
		case 'open':
			return 'text-danger';
		case 'investigating':
			return 'text-accent';
		default:
			return 'text-ink-mute';
	}
}

/** The three selectable incident statuses (for a status picker). */
export const INCIDENT_STATUSES: readonly string[] = ['open', 'investigating', 'resolved'] as const;

/** Whether an incident is open (`resolved_at is null` — the open-count gate). */
export function isIncidentOpen(incident: { resolved_at: string | null }): boolean {
	return incident.resolved_at == null;
}

/**
 * Whether an open incident is past its SLA (`sla_due_at` in the past). Resolved
 * incidents are never breaching; a null SLA never breaches. Injectable `now`
 * keeps it deterministic under test. (Mockup wrong-gate: open past SLA.)
 */
export function isSlaBreached(
	incident: { resolved_at: string | null; sla_due_at: string | null },
	now: Date = new Date()
): boolean {
	if (incident.resolved_at != null) return false;
	if (!incident.sla_due_at) return false;
	const due = new Date(incident.sla_due_at).getTime();
	if (Number.isNaN(due)) return false;
	return due < now.getTime();
}

// ── artifact audit strip status => the done-gate ─────────────────────────────

/**
 * The audit-view done-gate: the number of NON-dereferenced rows. `0` passes;
 * anything above renders the red-fail chip and blocks the compliance export
 * (spec: `non_dereferenced == 0`).
 */
export function nonDereferencedCount(rows: { dereferenced: boolean }[]): number {
	return rows.filter((r) => r.dereferenced === false).length;
}

/** Whether the audit view passes the strip gate (no non-dereferenced rows). */
export function auditPasses(rows: { dereferenced: boolean }[]): boolean {
	return nonDereferencedCount(rows) === 0;
}

/** Label for a row's strip status — the pass/red-fail chip text. */
export function stripStatusLabel(dereferenced: boolean): string {
	return dereferenced ? 'Stripped' : 'Source not dropped';
}

/** Token tone for a row's strip status: stripped reads success, else the danger red-fail. */
export function stripStatusToneClass(dereferenced: boolean): string {
	return dereferenced ? 'text-success' : 'text-danger';
}

/** The compliance-export columns the UI surfaces — the source-ref-free subset,
 * mirroring the backend `COMPLIANCE_COLUMNS` (api.rs:1440) exactly and in order.
 * By construction there is no source-reference column here. */
export const COMPLIANCE_COLUMNS: readonly string[] = [
	'artifact_id',
	'engagement_id',
	'client',
	'kind',
	'title',
	'dereferenced',
	'status',
	'published_at',
	'created_at'
] as const;

// ── internal ──────────────────────────────────────────────────────────────────

/** `some_raw_value` / `high` => `Some raw value` / `High`. Never throws. */
function titleize(s: string): string {
	if (!s) return '';
	const spaced = s.replace(/_/g, ' ');
	return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}
