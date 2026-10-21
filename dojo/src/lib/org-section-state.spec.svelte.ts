import { describe, expect, it, vi } from 'vitest';
import { OrgSectionState, type OrgActions } from '$lib/org-section-state.svelte';
import { DojoApiError } from '$lib/client-data';

// The org-zone console writes (#293): each one reloads on success and says why
// on failure; opening an incident that cannot be read opens nothing.

function actions(over: Partial<OrgActions> = {}): OrgActions {
	return {
		setRole: vi.fn().mockResolvedValue(undefined),
		reportIncident: vi.fn().mockResolvedValue(undefined),
		resolveIncident: vi.fn().mockResolvedValue(undefined),
		removeIncident: vi.fn().mockResolvedValue(undefined),
		openIncident: vi.fn().mockResolvedValue({ id: 'i1', title: 'outage' }),
		newEngagement: vi.fn().mockResolvedValue(undefined),
		closeEngagement: vi.fn().mockResolvedValue(undefined),
		removeEngagement: vi.fn().mockResolvedValue(undefined),
		removeViewer: vi.fn().mockResolvedValue(undefined),
		grantView: vi.fn().mockResolvedValue(undefined),
		revokeView: vi.fn().mockResolvedValue(undefined),
		saveTeam: vi.fn().mockResolvedValue(undefined),
		deleteTeam: vi.fn().mockResolvedValue(undefined),
		addTeamMember: vi.fn().mockResolvedValue(undefined),
		removeTeamMember: vi.fn().mockResolvedValue(undefined),
		grantTeamView: vi.fn().mockResolvedValue(undefined),
		...over
	} as OrgActions;
}
const apiError = (msg: string) => new DojoApiError(403, msg);

describe('OrgSectionState', () => {
	it('runs a write and reloads', async () => {
		const a = actions();
		const reload = vi.fn().mockResolvedValue(undefined);
		const s = new OrgSectionState(a, reload);
		await s.reportIncident('outage');
		expect(a.reportIncident).toHaveBeenCalledWith('outage');
		expect(reload).toHaveBeenCalledOnce();
		expect(s.actionError).toBeNull();
	});

	it("the dōjō's viewers: remove, grant and revoke a workspace, each reloading (#307 D8)", async () => {
		const a = actions();
		const reload = vi.fn().mockResolvedValue(undefined);
		const s = new OrgSectionState(a, reload);
		const v = { principalId: 'v1', name: null, email: 'v@x.co', workspaces: [] };
		await s.grantView(v, 'p1');
		await s.revokeView(v, 'p1');
		await s.removeViewer(v);
		expect(a.grantView).toHaveBeenCalledWith('v1', 'p1');
		expect(a.revokeView).toHaveBeenCalledWith('v1', 'p1');
		expect(a.removeViewer).toHaveBeenCalledWith('v1');
		expect(reload).toHaveBeenCalledTimes(3);
	});

	it("the dōjō's teams: save, members, a team-wide grant and delete, each reloading (#307 D7)", async () => {
		const a = actions();
		const reload = vi.fn().mockResolvedValue(undefined);
		const s = new OrgSectionState(a, reload);
		const t = { id: 't1', name: 'P', members: [] };
		expect(await s.saveTeam(null, 'Platform')).toBe(true);
		await s.addTeamMember(t, 'u1');
		await s.removeTeamMember(t, 'u1');
		await s.grantTeamView(t, 'p1');
		await s.deleteTeam(t);
		expect(a.saveTeam).toHaveBeenCalledWith(null, 'Platform');
		expect(a.addTeamMember).toHaveBeenCalledWith('t1', 'u1');
		expect(a.removeTeamMember).toHaveBeenCalledWith('t1', 'u1');
		expect(a.grantTeamView).toHaveBeenCalledWith('t1', 'p1');
		expect(a.deleteTeam).toHaveBeenCalledWith('t1');
		expect(reload).toHaveBeenCalledTimes(5);
	});

	it('a team save that fails says why and reports it did not save', async () => {
		const s = new OrgSectionState(actions({ saveTeam: vi.fn().mockRejectedValue(apiError('this dōjō already has a team named P')) }), vi.fn());
		expect(await s.saveTeam(null, 'P')).toBe(false);
		expect(s.actionError).toBe('this dōjō already has a team named P');
	});

	it('says why a write failed, and does not reload', async () => {
		const reload = vi.fn();
		const s = new OrgSectionState(actions({ closeEngagement: vi.fn().mockRejectedValue(apiError('only a lead can close')) }), reload);
		await s.closeEngagement({ id: 'e1' } as never);
		expect(s.actionError).toBe('only a lead can close');
		expect(reload).not.toHaveBeenCalled();
	});

	it('a failure that is not the dōjō’s still says something', async () => {
		const s = new OrgSectionState(actions({ removeIncident: vi.fn().mockRejectedValue(new TypeError('fetch failed')) }), vi.fn());
		await s.removeIncident({ id: 'i1' } as never);
		expect(s.actionError).toBe('that action could not be completed');
	});

	it('a member with no user id is not sent', async () => {
		const a = actions();
		const s = new OrgSectionState(a, vi.fn());
		await s.setRole({ userId: null } as never, 'lead');
		expect(a.setRole).not.toHaveBeenCalled();
		await s.setRole({ userId: 'u1' } as never, 'lead');
		expect(a.setRole).toHaveBeenCalledWith('u1', 'lead');
	});

	it('opens an incident’s detail, and a failed read opens nothing', async () => {
		const s = new OrgSectionState(actions(), vi.fn());
		await s.openIncident({ id: 'i1' } as never);
		expect(s.incidentDetail).toMatchObject({ id: 'i1' });
		s.closeIncident();
		expect(s.incidentDetail).toBeNull();

		const f = new OrgSectionState(actions({ openIncident: vi.fn().mockRejectedValue(apiError('gone')) }), vi.fn());
		await f.openIncident({ id: 'i2' } as never);
		expect(f.incidentDetail).toBeNull();
		expect(f.actionError).toBe('gone');
	});

	it('re-reads the open incident after resolving it, so the pane is not stale', async () => {
		const openIncident = vi
			.fn()
			.mockResolvedValueOnce({ id: 'i1', status: 'open' })
			.mockResolvedValueOnce({ id: 'i1', status: 'resolved' });
		const s = new OrgSectionState(actions({ openIncident }), vi.fn().mockResolvedValue(undefined));
		await s.openIncident({ id: 'i1' } as never);
		await s.resolveIncident({ id: 'i1' } as never);
		expect(s.incidentDetail).toMatchObject({ status: 'resolved' });
	});

	it('deleting the open incident closes its pane', async () => {
		const s = new OrgSectionState(actions(), vi.fn().mockResolvedValue(undefined));
		await s.openIncident({ id: 'i1' } as never);
		await s.removeIncident({ id: 'i1' } as never);
		expect(s.incidentDetail).toBeNull();
	});

	it('another incident’s write leaves the open pane alone', async () => {
		const a = actions();
		const s = new OrgSectionState(a, vi.fn().mockResolvedValue(undefined));
		await s.openIncident({ id: 'i1' } as never);
		await s.removeIncident({ id: 'i2' } as never);
		expect(s.incidentDetail).toMatchObject({ id: 'i1' });
		expect(a.openIncident).toHaveBeenCalledOnce();
	});
});
