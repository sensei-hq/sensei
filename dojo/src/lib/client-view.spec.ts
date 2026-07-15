import { describe, expect, it } from 'vitest';
import {
	auditPasses,
	bindingsSummary,
	engagementStatusLabel,
	engagementStatusToneClass,
	incidentStatusLabel,
	incidentStatusToneClass,
	isIncidentOpen,
	isSlaBreached,
	nonDereferencedCount,
	projectBindings,
	severityLabel,
	severityRank,
	severityToneClass,
	stripStatusLabel,
	stripStatusToneClass,
	COMPLIANCE_COLUMNS
} from '$lib/client-view';

describe('engagement status', () => {
	it('labels active / ended and titles an unknown value', () => {
		expect(engagementStatusLabel('active')).toBe('Active');
		expect(engagementStatusLabel('ended')).toBe('Ended');
		expect(engagementStatusLabel('on_hold')).toBe('On hold');
	});

	it('tones active as success and ended as muted', () => {
		expect(engagementStatusToneClass('active')).toBe('text-success');
		expect(engagementStatusToneClass('ended')).toBe('text-ink-mute');
	});
});

describe('project bindings', () => {
	it('normalises well-formed bindings and skips malformed entries', () => {
		const eng = {
			project_bindings: [
				{ project_id: 'p1', name: 'ledger-core' },
				{ project_id: 'p2' }, // no name
				{ name: 'orphan' }, // no project_id => skipped
				{ project_id: '' }, // empty id => skipped
				'nonsense' // not an object => skipped
			]
		};
		expect(projectBindings(eng)).toEqual([
			{ project_id: 'p1', name: 'ledger-core' },
			{ project_id: 'p2', name: null }
		]);
	});

	it('returns [] when project_bindings is not an array', () => {
		expect(projectBindings({ project_bindings: {} })).toEqual([]);
		expect(projectBindings({ project_bindings: null })).toEqual([]);
	});

	it('summarises bindings using name then id, — when empty', () => {
		expect(
			bindingsSummary({ project_bindings: [{ project_id: 'p1', name: 'auth' }, { project_id: 'p2' }] })
		).toBe('auth · p2');
		expect(bindingsSummary({ project_bindings: [] })).toBe('—');
	});
});

describe('incident severity', () => {
	it('ranks worst-first (critical > high > medium > low > unknown)', () => {
		expect(severityRank('critical')).toBeGreaterThan(severityRank('high'));
		expect(severityRank('high')).toBeGreaterThan(severityRank('medium'));
		expect(severityRank('medium')).toBeGreaterThan(severityRank('low'));
		expect(severityRank('low')).toBeGreaterThan(severityRank('mystery'));
	});

	it('labels each severity', () => {
		expect(severityLabel('critical')).toBe('Critical');
		expect(severityLabel('low')).toBe('Low');
	});

	it('tones critical/high as danger, medium accent, low muted', () => {
		expect(severityToneClass('critical')).toBe('text-danger');
		expect(severityToneClass('high')).toBe('text-danger');
		expect(severityToneClass('medium')).toBe('text-accent');
		expect(severityToneClass('low')).toBe('text-ink-soft');
	});
});

describe('incident status + open/SLA', () => {
	it('labels + tones each status', () => {
		expect(incidentStatusLabel('investigating')).toBe('Investigating');
		expect(incidentStatusToneClass('resolved')).toBe('text-success');
		expect(incidentStatusToneClass('open')).toBe('text-danger');
		expect(incidentStatusToneClass('investigating')).toBe('text-accent');
	});

	it('treats a null resolved_at as open', () => {
		expect(isIncidentOpen({ resolved_at: null })).toBe(true);
		expect(isIncidentOpen({ resolved_at: '2026-07-01T00:00:00Z' })).toBe(false);
	});

	it('breaches SLA only for an open incident with a past due date', () => {
		const now = new Date('2026-07-10T00:00:00Z');
		// open + past due => breach
		expect(
			isSlaBreached({ resolved_at: null, sla_due_at: '2026-07-01T00:00:00Z' }, now)
		).toBe(true);
		// open + future due => no breach
		expect(
			isSlaBreached({ resolved_at: null, sla_due_at: '2026-07-20T00:00:00Z' }, now)
		).toBe(false);
		// resolved => never breaches even if past due
		expect(
			isSlaBreached(
				{ resolved_at: '2026-07-05T00:00:00Z', sla_due_at: '2026-07-01T00:00:00Z' },
				now
			)
		).toBe(false);
		// no SLA => no breach
		expect(isSlaBreached({ resolved_at: null, sla_due_at: null }, now)).toBe(false);
	});
});

describe('artifact audit strip gate (non_dereferenced == 0)', () => {
	it('counts non-dereferenced rows as the red-fail count', () => {
		const rows = [
			{ dereferenced: true },
			{ dereferenced: false },
			{ dereferenced: true },
			{ dereferenced: false }
		];
		expect(nonDereferencedCount(rows)).toBe(2);
	});

	it('passes only when every row is dereferenced (spec: non_dereferenced == 0)', () => {
		expect(auditPasses([{ dereferenced: true }, { dereferenced: true }])).toBe(true);
		expect(auditPasses([])).toBe(true);
		// a single non-dereferenced row fails the gate
		expect(auditPasses([{ dereferenced: true }, { dereferenced: false }])).toBe(false);
	});

	it('labels + tones a stripped row vs a non-dereferenced red-fail', () => {
		expect(stripStatusLabel(true)).toBe('Stripped');
		expect(stripStatusToneClass(true)).toBe('text-success');
		expect(stripStatusLabel(false)).toBe('Source not dropped');
		expect(stripStatusToneClass(false)).toBe('text-danger');
	});
});

describe('compliance export columns are the source-ref-free subset', () => {
	it('mirrors the backend COMPLIANCE_COLUMNS in order and never names a source ref', () => {
		expect([...COMPLIANCE_COLUMNS]).toEqual([
			'artifact_id',
			'engagement_id',
			'client',
			'kind',
			'title',
			'dereferenced',
			'status',
			'published_at',
			'created_at'
		]);
		// no source-reference column may leak into the export
		for (const forbidden of [
			'contributed_by',
			'attribution',
			'signature',
			'approved_by',
			'payload',
			'scope',
			'body',
			'source',
			'source_ref'
		]) {
			expect(COMPLIANCE_COLUMNS).not.toContain(forbidden);
		}
	});
});
