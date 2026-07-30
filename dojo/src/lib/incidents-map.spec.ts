// The dojo org lead Incidents + Client-audit wire→kit mappers. Deterministic `now`.
import { describe, expect, it } from 'vitest';
import type { Incident } from './client-data';
import type { AuditEvent } from './admin-data';
import {
	incidentState,
	toKitIncident,
	toKitIncidents,
	entryHeld,
	toKitClientAuditRow,
	toKitClientAudit
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
