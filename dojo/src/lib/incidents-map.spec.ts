// The dojo org lead Incidents + Client-audit wire→kit mappers. Deterministic `now`.
import { describe, expect, it } from 'vitest';
import type { Incident, IncidentDetail } from './client-data';
import type { AuditEvent } from './admin-data';
import {
	incidentState,
	toKitIncident,
	toKitIncidents,
	toKitIncidentDetail,
	entryHeld,
	toKitClientAuditRow,
	toKitClientAudit,
	toKitClientAuditLedgerRow,
	toKitClientAuditLedger
} from './incidents-map';

const NOW = new Date('2026-07-23T12:00:00Z');

function inc(over: Partial<Incident> = {}): Incident {
	return {
		id: 'i1',
		engagement_id: 'eng-12345678-aaaa',
		client_name: null,
		artifact_id: null,
		title: 'Near-leak: client hostname in a shared prompt',
		description: null,
		severity: 'high',
		status: 'investigating',
		owner_id: null,
		sla_due_at: null,
		resolution: null,
		opened_at: '2026-07-20T12:00:00Z',
		resolved_at: null,
		...over
	};
}

describe('incidentState', () => {
	it('resolved when resolved_at set or status resolved', () => {
		expect(incidentState(inc({ resolved_at: '2026-07-21T00:00:00Z' }))).toBe('resolved');
		expect(incidentState(inc({ status: 'resolved', resolved_at: null }))).toBe('resolved');
	});
	it('contained when investigating (leak-guard held it)', () => {
		expect(incidentState(inc({ status: 'investigating' }))).toBe('contained');
	});
	it('open otherwise', () => {
		expect(incidentState(inc({ status: 'open' }))).toBe('open');
	});
});

describe('toKitIncident / toKitIncidents', () => {
	it('maps the row onto the kit incident', () => {
		const k = toKitIncident(inc(), NOW);
		expect(k.id).toBe('i1');
		expect(k.kanji).toBe('盾');
		expect(k.title).toMatch(/near-leak/i);
		expect(k.severity).toBe('high');
		expect(k.state).toBe('contained');
		expect(k.when).toBe('3d');
		expect(k.client).toBe('eng-1234'); // short engagement id (no resolved name)
	});
	it('prefers the resolved client_name over the short engagement id', () => {
		expect(toKitIncident(inc({ client_name: 'Globex' }), NOW).client).toBe('Globex');
	});
	it('client "—" when unbound', () => {
		expect(toKitIncident(inc({ engagement_id: null }), NOW).client).toBe('—');
	});
	it('preserves order', () => {
		const out = toKitIncidents([inc({ id: 'a' }), inc({ id: 'b' })], NOW);
		expect(out.map((i) => i.id)).toEqual(['a', 'b']);
	});
});

function detail(over: Partial<IncidentDetail> = {}): IncidentDetail {
	return { ...inc(), owner_name: null, owner_email: null, artifact: null, ...over };
}

describe('toKitIncidentDetail', () => {
	it('maps the detail with resolved client, owner, sla, resolution, and linked artifact', () => {
		const k = toKitIncidentDetail(
			detail({
				client_name: 'Globex',
				owner_name: 'Ada',
				sla_due_at: '2026-08-01',
				resolution: 'contained — source stripped before it left',
				artifact: { id: 'a1', title: 'the pattern', kind: 'pattern', status: 'archived' }
			}),
			NOW
		);
		expect(k.client).toBe('Globex');
		expect(k.owner).toBe('Ada');
		expect(k.state).toBe('contained'); // status investigating
		expect(k.sla).toBe('2026-08-01');
		expect(k.resolution).toMatch(/source stripped/);
		expect(k.artifact).toEqual({ title: 'the pattern', kind: 'pattern', status: 'archived' });
	});
	it('falls back owner "—" and artifact null when absent', () => {
		const k = toKitIncidentDetail(detail(), NOW);
		expect(k.owner).toBe('—');
		expect(k.artifact).toBeNull();
		expect(k.client).toBe('eng-1234'); // short engagement id (no resolved name)
	});
});

function ev(over: Partial<AuditEvent> = {}): AuditEvent {
	return {
		id: 1,
		ts: '2026-07-23T10:42:00Z',
		actor_id: 'user-1',
		engagement_id: 'eng-12345678-bbbb',
		action: 'publish',
		target: 'idempotency pattern',
		detail: {},
		...over
	};
}

describe('entryHeld', () => {
	it('false for block/decline actions, true otherwise', () => {
		expect(entryHeld('blocked contribution')).toBe(false);
		expect(entryHeld('decline')).toBe(false);
		expect(entryHeld('publish')).toBe(true);
		expect(entryHeld('strip')).toBe(true);
	});
});

describe('toKitClientAuditRow / toKitClientAudit', () => {
	it('maps a share event to a held ledger row', () => {
		const r = toKitClientAuditRow(ev(), NOW);
		expect(r.event).toBe('publish');
		expect(r.detail).toBe('idempotency pattern');
		expect(r.client).toBe('eng-1234');
		expect(r.ok).toBe(true);
		expect(r.kanji).toBe('共');
		expect(r.t).toMatch(/h|m|just now/);
	});
	it('maps a block event to a not-held row (却)', () => {
		const r = toKitClientAuditRow(ev({ action: 'blocked contribution', target: null }), NOW);
		expect(r.ok).toBe(false);
		expect(r.kanji).toBe('却');
	});
	it('falls back to JSON detail when no target', () => {
		const r = toKitClientAuditRow(ev({ target: null, detail: { stripped: 2 } }), NOW);
		expect(r.detail).toBe('{"stripped":2}');
	});
	it('preserves order', () => {
		const out = toKitClientAudit([ev({ id: 1 }), ev({ id: 2 })], NOW);
		expect(out).toHaveLength(2);
	});
});

describe('toKitClientAuditLedger (confidentiality ledger)', () => {
	function led(over: Partial<import('./client-data').ClientAuditEntry> = {}): import('./client-data').ClientAuditEntry {
		return {
			id: 1,
			ts: '2026-07-23T10:00:00Z',
			action: 'publish',
			target: 'idempotency pattern',
			detail: null,
			engagement_id: 'eng-12345678',
			client_name: 'Globex',
			...over
		};
	}
	it('maps a publish entry: "Lesson shared upstream", real client name, ok=true', () => {
		const r = toKitClientAuditLedgerRow(led(), NOW);
		expect(r.event).toBe('Lesson shared upstream');
		expect(r.client).toBe('Globex'); // resolved client name (Rule C), not a uuid
		expect(r.ok).toBe(true);
		expect(r.kanji).toBe('共');
	});
	it('maps a contained entry: "Near-leak contained" and still ok=true (the guard working)', () => {
		const r = toKitClientAuditLedgerRow(led({ action: 'contained', client_name: null }), NOW);
		expect(r.event).toBe('Near-leak contained');
		expect(r.ok).toBe(true);
		expect(r.client).toBe('eng-1234'); // short engagement id when no name resolved
	});
	it('falls back to JSON detail when no target', () => {
		expect(toKitClientAuditLedgerRow(led({ target: null, detail: { held: 1 } }), NOW).detail).toBe('{"held":1}');
	});
	it('preserves order', () => {
		expect(toKitClientAuditLedger([led({ id: 1 }), led({ id: 2 })], NOW)).toHaveLength(2);
	});
});
