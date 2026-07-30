// Pure wire→kit mappers for the org lead Incidents + Client-audit screens (dojo
// `/org/[slug]/incidents` · `/clientaudit`). Take the `client-data.ts` `Incident`
// wire row and the `admin-data.ts` `AuditEvent` row (the SAME rows the shipped
// `(console)` screens fetch) and project them onto the presentational
// `KitIncident[]` / `KitClientAuditRow[]` the dojo screens declare.
// Side-effect-free so they're DRY + unit-tested once; `now` is injected.
//
// On the client-audit source: the tenant's immutable confidentiality ledger is
// the tenant-scoped `dojo.audit_events` (what GET …/audit serves); this is the
// same trail the admin Audit tab reads, projected to the ledger row shape. A
// dedicated per-engagement artifact-strip ledger (GET …/audit/artifacts) is a
// follow-on when that panel's engagement selector is wired.

import type { Incident } from './client-data';
import type { AuditEvent } from './admin-data';
import { relativeAge } from './triage/view';
import type { KitIncident, KitClientAuditRow } from './components/kit/types';

/** The incident glyph (盾 — "shield"), matching the fixture. */
const INCIDENT_KANJI = '盾';

/**
 * The display state the mockup's state dot expects (`contained` · `resolved` ·
 * `open`). The wire status is `open | investigating | resolved`; an
 * investigating-but-held incident reads `contained` (the leak-guard held it),
 * matching the mockup's vocabulary. Pure.
 */
export function incidentState(i: Incident): string {
	if (i.resolved_at != null || i.status === 'resolved') return 'resolved';
	if (i.status === 'investigating') return 'contained';
	return 'open';
}

/**
 * Incident → KitIncident. `client` is the resolved client name (the GET route
 * joins engagement_id → engagements.client_name), falling back to a short
 * engagement id, then "—" when unbound; `when` is the relative opened-at age. Pure.
 */
export function toKitIncident(i: Incident, now: Date = new Date()): KitIncident {
	return {
		id: i.id,
		kanji: INCIDENT_KANJI,
		title: i.title,
		client: i.client_name ?? (i.engagement_id ? i.engagement_id.slice(0, 8) : '—'),
		state: incidentState(i),
		when: relativeAge(i.opened_at, now),
		severity: i.severity
	};
}

/** Incident[] → KitIncident[], preserving the API's worst-first order. Pure. */
export function toKitIncidents(incidents: Incident[], now: Date = new Date()): KitIncident[] {
	return incidents.map((i) => toKitIncident(i, now));
}

// ── audit events → the client-audit ledger ───────────────────────────────────

/** A glyph for a ledger row by action: a block/decline reads 却 ("reject"), a
 *  share reads 共 ("share"), anything else the neutral 録 ("record"). */
function ledgerKanji(action: string): string {
	if (/block|decline|reject|deny/i.test(action)) return '却';
	if (/publish|share|distribut/i.test(action)) return '共';
	return '録';
}

/** Whether the entry proves confidentiality held (`ok`) — a block/decline is a
 *  false (a contribution was held), everything else held. */
export function entryHeld(action: string): boolean {
	return !/block|decline|reject|deny/i.test(action);
}

/**
 * AuditEvent → KitClientAuditRow. `t` is the relative age; `event` is the action;
 * `detail` is the target (or a compact JSON of the detail object); `client` is
 * the short engagement id (or "—"); `ok` reflects whether confidentiality held.
 * Pure.
 */
export function toKitClientAuditRow(e: AuditEvent, now: Date = new Date()): KitClientAuditRow {
	let detail = e.target ?? '';
	if (!detail && e.detail && typeof e.detail === 'object') {
		try {
			detail = JSON.stringify(e.detail);
		} catch {
			detail = '';
		}
	}
	return {
		t: relativeAge(e.ts, now),
		kanji: ledgerKanji(e.action),
		event: e.action,
		detail,
		client: e.engagement_id ? e.engagement_id.slice(0, 8) : '—',
		ok: entryHeld(e.action)
	};
}

/** AuditEvent[] → KitClientAuditRow[], preserving the API's (most-recent-first)
 *  order. Pure. */
export function toKitClientAudit(events: AuditEvent[], now: Date = new Date()): KitClientAuditRow[] {
	return events.map((e) => toKitClientAuditRow(e, now));
}
