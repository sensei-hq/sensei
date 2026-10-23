// The Teams view (#307 R8, D38): every member reads their teams' totals; a lead
// also reads the people on their teams, an admin every team and its people. The
// database decides whose numbers come back — dojo.team_member_metrics runs as the
// caller (repository_metrics' policy), dojo.team_metric_totals answers only
// totals over at least k people — this module picks whom to ask and shapes. A
// failed read is an error, never an empty view.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, it, expect } from 'vitest';
import { fakeDojoDb, type FakeTable } from './fake-dojo-db';
import { readTeamMetrics, windowSince } from './team-metrics-data';
import { AdminError, type DojoClient } from './admin-data';
import { K_ANONYMITY_MIN_CONTRIBUTORS } from './artifacts-data';
import { TEAM_METRIC_KEYS } from '$lib/team-metrics';

const T = 'tenant-t';
const TODAY = new Date('2026-10-22T09:00:00Z');
const opts = { days: 7, today: TODAY };

function db(over: Record<string, FakeTable> = {}) {
	const f = fakeDojoDb({
		teams: {
			rows: [
				{ id: 'tmA', tenant_id: T, name: 'Payments' },
				{ id: 'tmB', tenant_id: T, name: 'Search' },
				{ id: 'tmC', tenant_id: T, name: 'Core' }
			]
		},
		team_members: {
			rows: [
				{ team_id: 'tmA', principal_id: 'lena' },
				{ team_id: 'tmA', principal_id: 'mo' },
				{ team_id: 'tmC', principal_id: 'lena' },
				{ team_id: 'tmC', principal_id: 'mo' },
				{ team_id: 'tmC', principal_id: 'mia' },
				{ team_id: 'tmB', principal_id: 'ned' }
			]
		},
		memberships: {
			rows: ['lena', 'mo', 'mia', 'ned'].map((user_id) => ({ tenant_id: T, user_id }))
		},
		identities: {
			rows: [
				{ principal_id: 'lena', display_name: 'Lena', email: null, last_login_at: null },
				{ principal_id: 'mo', display_name: 'Mo', email: null, last_login_at: null }
			]
		},
		'rpc:team_member_metrics': {
			rows: [
				{ principal_id: 'mo', metric_key: 'throughput', total: 12, weighted: null, days: 4, repositories: 2, last_on: '2026-10-21' },
				{ principal_id: 'mo', metric_key: 'ftr', total: 3.2, weighted: 0.75, days: 4, repositories: 2, last_on: '2026-10-21' },
				{ principal_id: 'lena', metric_key: 'throughput', total: 5, weighted: null, days: 1, repositories: 1, last_on: '2026-10-20' }
			]
		},
		'rpc:team_metric_totals': {
			rows: [
				{ team_id: 'tmC', metric_key: 'throughput', total: 30, weighted: null, contributors: 3 },
				{ team_id: 'tmC', metric_key: 'ftr', total: '2.1', weighted: '0.7', contributors: 3 }
			]
		},
		...over
	});
	return f as unknown as DojoClient & typeof f;
}

const lead = { tenantId: T, userId: 'lena', role: 'lead' };

describe('windowSince', () => {
	it('is the first day of a window ending today, as a date', () => {
		expect(windowSince(TODAY, 7)).toBe('2026-10-16');
		expect(windowSince(TODAY, 1)).toBe('2026-10-22');
	});
});

describe('readTeamMetrics — a lead', () => {
	it('lists only the teams the lead is on, each with its totals and its people', async () => {
		const view = await readTeamMetrics(db(), db(), lead, opts);
		expect(view).toMatchObject({ since: '2026-10-16', days: 7, per_person: true, k_min: 3 });
		expect(view.teams.map((t) => [t.name, t.member_count])).toEqual([
			['Core', 3],
			['Payments', 2]
		]);
		expect(view.teams[0].totals).toEqual({
			throughput: { total: 30, weighted: null, contributors: 3 },
			ftr: { total: 2.1, weighted: 0.7, contributors: 3 }
		});
		expect(view.teams[1].totals).toEqual({});
		const payments = view.teams[1];
		expect(payments.members.map((m) => [m.principal_id, m.display_name, m.you])).toEqual([
			['lena', 'Lena', true],
			['mo', 'Mo', false]
		]);
		expect(payments.members[1].metrics.ftr).toEqual({
			total: 3.2,
			weighted: 0.75,
			days: 4,
			repositories: 2,
			last_on: '2026-10-21'
		});
		expect(view.teams[0].members.find((m) => m.principal_id === 'mia')?.metrics).toEqual({});
	});

	it('asks for the people on its teams only — never someone on a team it is not on', async () => {
		const d = db();
		await readTeamMetrics(d, d, lead, opts);
		const perPerson = d.rpcCalls.find((c) => c.fn === 'team_member_metrics');
		expect(perPerson?.args).toEqual({
			p_tenant: T,
			p_principals: ['lena', 'mo', 'mia'],
			p_keys: [...TEAM_METRIC_KEYS],
			p_since: '2026-10-16'
		});
		expect(d.rpcCalls.find((c) => c.fn === 'team_metric_totals')?.args).toEqual({
			p_tenant: T,
			p_keys: [...TEAM_METRIC_KEYS],
			p_since: '2026-10-16'
		});
	});

	it('on no team: no team, and the database is not asked', async () => {
		const d = db();
		const view = await readTeamMetrics(d, d, { ...lead, userId: 'zed' }, opts);
		expect(view.teams).toEqual([]);
		expect(d.rpcCalls).toEqual([]);
	});
});

describe('readTeamMetrics — an admin', () => {
	it('lists every team of the dōjō with its people', async () => {
		const d = db();
		const view = await readTeamMetrics(d, d, { tenantId: T, userId: 'zed', role: 'admin' }, opts);
		expect(view.per_person).toBe(true);
		expect(view.teams.map((t) => t.name)).toEqual(['Core', 'Payments', 'Search']);
		expect(d.rpcCalls.find((c) => c.fn === 'team_member_metrics')?.args.p_principals).toEqual([
			'lena',
			'mo',
			'mia',
			'ned'
		]);
	});
});

describe('readTeamMetrics — below lead', () => {
	for (const role of ['contributor', 'maintainer']) {
		it(`a ${role} gets their teams' totals and no person`, async () => {
			const d = db();
			const view = await readTeamMetrics(d, d, { tenantId: T, userId: 'mo', role }, opts);
			expect(view.per_person).toBe(false);
			expect(view.teams.map((t) => t.name)).toEqual(['Core', 'Payments']);
			expect(view.teams.every((t) => t.members.length === 0)).toBe(true);
			expect(view.teams[0].totals.throughput).toEqual({ total: 30, weighted: null, contributors: 3 });
			expect(d.rpcCalls.map((c) => c.fn)).toEqual(['team_metric_totals']);
		});
	}
});

describe('readTeamMetrics — failures', () => {
	it('a failed read is an error, never an empty view', async () => {
		for (const broken of ['rpc:team_member_metrics', 'rpc:team_metric_totals', 'teams']) {
			const d = db({ [broken]: { rows: [], error: { message: 'boom' } } });
			await expect(readTeamMetrics(d, d, lead, opts)).rejects.toThrow(AdminError);
		}
	});

	it("names teammates through the name client, not the caller's", async () => {
		const caller = db({ identities: { rows: [] } });
		const view = await readTeamMetrics(caller, db(), lead, opts);
		expect(view.teams[1].members.map((m) => m.display_name)).toEqual(['Lena', 'Mo']);
		expect(caller.tables.identities.reads).toBe(0);
	});
});

describe('the k-anonymity floor', () => {
	it('is one number: the database function returns the knowledge gate’s floor', () => {
		const ddl = readFileSync(resolve(process.cwd(), '../database/ddl/function/dojo/k_anonymity_min.ddl'), 'utf8');
		const body = ddl.match(/as \$\$\s*select (\d+);\s*\$\$/);
		expect(Number(body?.[1])).toBe(K_ANONYMITY_MIN_CONTRIBUTORS);
	});
});
