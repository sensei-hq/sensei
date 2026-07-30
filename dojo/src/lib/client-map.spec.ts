import { describe, expect, it } from 'vitest';
import type { Engagement } from './client-data';
import { toKitEngagement, toKitEngagements, bindingsLabel } from './client-map';

// The dojo org lead Engagements wire→kit mapper. Deterministic `now` for `since`.

const NOW = new Date('2026-07-23T12:00:00Z');

function engagement(over: Partial<Engagement> = {}): Engagement {
	return {
		id: 'e1',
		client_name: 'Globex',
		client_tenant_id: null,
		description: 'portal + billing',
		project_bindings: [{ project_id: 'p1', name: 'globex-portal' }, { project_id: 'p2', name: 'billing' }],
		policy_overrides: {},
		status: 'active',
		starts_on: '2026-01-23',
		ends_on: null,
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-07-01T00:00:00Z',
		lessons_kept: 0,
		stripped: 0,
		...over
	};
}

describe('bindingsLabel', () => {
	it('joins bound project names with " · "', () => {
		expect(bindingsLabel([{ name: 'a' }, { name: 'b' }])).toBe('a · b');
	});
	it('falls back to project_id when a binding has no name', () => {
		expect(bindingsLabel([{ project_id: 'p9' }])).toBe('p9');
	});
	it('returns "—" for an empty or non-array binding set', () => {
		expect(bindingsLabel([])).toBe('—');
		expect(bindingsLabel(null)).toBe('—');
		expect(bindingsLabel('nope')).toBe('—');
	});
});

describe('toKitEngagement / toKitEngagements', () => {
	it('maps the engagement onto the kit row', () => {
		const k = toKitEngagement(engagement(), NOW);
		expect(k.id).toBe('e1');
		expect(k.kanji).toBe('客');
		expect(k.client).toBe('Globex');
		expect(k.projects).toBe('globex-portal · billing');
		expect(k.status).toBe('active');
		expect(k.since).toBe('181d'); // relativeAge(2026-01-23 → 2026-07-23) in days
	});

	it('derives "since" from starts_on when present, else created_at', () => {
		expect(toKitEngagement(engagement({ starts_on: '2026-07-16' }), NOW).since).toBe('7d');
		expect(toKitEngagement(engagement({ starts_on: null, created_at: '2026-07-22T12:00:00Z' }), NOW).since).toBe('1d');
	});

	it('maps the real kept/stripped artifact counts through', () => {
		const k = toKitEngagement(engagement({ lessons_kept: 5, stripped: 2 }), NOW);
		expect(k.lessons).toBe(5);
		expect(k.dropped).toBe(2);
	});

	it('carries client_tenant_id for the client link (null when the client is not a known tenant)', () => {
		expect(toKitEngagement(engagement({ client_tenant_id: null }), NOW).clientTenantId).toBeNull();
		expect(toKitEngagement(engagement({ client_tenant_id: 'ct-9' }), NOW).clientTenantId).toBe('ct-9');
	});

	it('toKitEngagements preserves order', () => {
		const rows = toKitEngagements([engagement({ id: 'a', client_name: 'A' }), engagement({ id: 'b', client_name: 'B' })], NOW);
		expect(rows.map((r) => r.client)).toEqual(['A', 'B']);
	});
});
