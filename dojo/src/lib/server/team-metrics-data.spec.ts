// The lead's team view (#307 R8, D38): the teams the caller is ON, each member
// with their per-person metric totals over a window. The database decides whose
// numbers come back (dojo.team_member_metrics runs as the caller, under
// repository_metrics' policy); this module only asks and shapes. A lead on no team
// sees no team. A failed read is an error, never an empty view.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb, type FakeTable } from './fake-dojo-db';
import { readTeamMetrics, windowSince } from './team-metrics-data';
import { AdminError, type DojoClient } from './admin-data';
import { TEAM_METRIC_KEYS } from '$lib/team-metrics';

const T = 'tenant-t';
const TODAY = new Date('2026-10-22T09:00:00Z');

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
				{ principal_id: 'mo', metric_key: 'throughput', total: 12, mean: 3, days: 4, repositories: 2, last_on: '2026-10-21' },
				{ principal_id: 'mo', metric_key: 'ftr', total: 3.2, mean: 0.8, days: 4, repositories: 2, last_on: '2026-10-21' },
				{ principal_id: 'lena', metric_key: 'throughput', total: 5, mean: 5, days: 1, repositories: 1, last_on: '2026-10-20' }
			]
		},
		...over
	});
	return f as unknown as DojoClient & typeof f;
}

describe('windowSince', () => {
	it('is the first day of a window ending today, as a date', () => {
		expect(windowSince(TODAY, 7)).toBe('2026-10-16');
		expect(windowSince(TODAY, 1)).toBe('2026-10-22');
	});
});

describe('readTeamMetrics', () => {
	it("lists only the teams the caller is on, by name, each member with the numbers the database returned", async () => {
		const d = db();
		const view = await readTeamMetrics(d, d, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY });
		expect(view.since).toBe('2026-10-16');
		expect(view.days).toBe(7);
		expect(view.teams.map((t) => t.name)).toEqual(['Core', 'Payments']);
		const payments = view.teams[1];
		expect(payments.members.map((m) => [m.principal_id, m.display_name, m.you])).toEqual([
			['lena', 'Lena', true],
			['mo', 'Mo', false]
		]);
		expect(payments.members[1].metrics).toEqual({
			throughput: { total: 12, mean: 3, days: 4, repositories: 2, last_on: '2026-10-21' },
			ftr: { total: 3.2, mean: 0.8, days: 4, repositories: 2, last_on: '2026-10-21' }
		});
		// mia is on Core; the database returned nothing readable for her.
		expect(view.teams[0].members.find((m) => m.principal_id === 'mia')?.metrics).toEqual({});
	});

	it('asks the database once, as the caller, for every teammate and the shown metrics since the window start', async () => {
		const d = db();
		await readTeamMetrics(d, d, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY });
		expect(d.rpcCalls).toEqual([
			{
				fn: 'team_member_metrics',
				args: {
					p_tenant: T,
					p_principals: ['lena', 'mo', 'mia'],
					p_keys: [...TEAM_METRIC_KEYS],
					p_since: '2026-10-16'
				}
			}
		]);
	});

	it("never asks about someone on a team the caller is not on", async () => {
		const d = db();
		await readTeamMetrics(d, d, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY });
		expect(d.rpcCalls[0].args.p_principals).not.toContain('ned');
	});

	it('a caller on no team sees no team and the database is not asked', async () => {
		const d = db();
		const view = await readTeamMetrics(d, d, { tenantId: T, userId: 'zed' }, { days: 7, today: TODAY });
		expect(view.teams).toEqual([]);
		expect(d.rpcCalls).toEqual([]);
	});

	it('a failed read is an error, never an empty view', async () => {
		const failing = db({ 'rpc:team_member_metrics': { rows: [], error: { message: 'boom' } } });
		await expect(readTeamMetrics(failing, failing, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY })).rejects.toThrow(
			AdminError
		);
		const noTeams = db({ teams: { rows: [], error: { message: 'boom' } } });
		await expect(readTeamMetrics(noTeams, noTeams, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY })).rejects.toThrow(
			AdminError
		);
	});

	it("names teammates through the name client, not the caller's (who may not read identities)", async () => {
		const caller = db({ identities: { rows: [] } });
		const names = db();
		const view = await readTeamMetrics(caller, names, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY });
		expect(view.teams[1].members.map((m) => m.display_name)).toEqual(['Lena', 'Mo']);
		expect(caller.tables.identities.reads).toBe(0);
	});

	it('a number the database sends as text is read as a number', async () => {
		const d = db({
			'rpc:team_member_metrics': {
				rows: [{ principal_id: 'mo', metric_key: 'ftr', total: '1.5', mean: '0.75', days: 2, repositories: 1, last_on: '2026-10-21' }]
			}
		});
		const view = await readTeamMetrics(d, d, { tenantId: T, userId: 'lena' }, { days: 7, today: TODAY });
		const mo = view.teams[1].members[1];
		expect(mo.metrics.ftr).toEqual({ total: 1.5, mean: 0.75, days: 2, repositories: 1, last_on: '2026-10-21' });
	});
});
