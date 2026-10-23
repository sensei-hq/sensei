// The org-zone console writes (#293) — a member's role, the dōjō's viewers and
// teams. The page binds each write to the org's tenant
// and session; this runs it, reloads on success, and says why on failure.
import { DojoApiError } from '$lib/client-data';
import type { KitMember } from '$lib/components/kit/types';
import type { DojoViewerView } from '$lib/viewers-view';
import type { TeamView } from '$lib/teams-view';

/** The console writes, already bound to the org's tenant and session token. */
export interface OrgActions {
	setRole: (userId: string, role: string) => Promise<unknown>;
	/** The dōjō's viewers (#307 D8). */
	removeViewer: (principalId: string) => Promise<unknown>;
	grantView: (principalId: string, workspaceId: string) => Promise<unknown>;
	revokeView: (principalId: string, workspaceId: string) => Promise<unknown>;
	/** The dōjō's teams (#307 D7). `teamId` null creates. */
	saveTeam: (teamId: string | null, name: string) => Promise<unknown>;
	deleteTeam: (teamId: string) => Promise<unknown>;
	addTeamMember: (teamId: string, principalId: string) => Promise<unknown>;
	removeTeamMember: (teamId: string, principalId: string) => Promise<unknown>;
	grantTeamView: (teamId: string, workspaceId: string) => Promise<unknown>;
}

/** Re-read the section's load (the page passes invalidateAll). */
export type Reload = () => Promise<unknown>;

const message = (e: unknown, fallback: string) => (e instanceof DojoApiError ? e.message : fallback);

export class OrgSectionState {
	/** Why the last action failed — shown under the screen, never swallowed. */
	actionError = $state<string | null>(null);

	constructor(
		private readonly actions: OrgActions,
		private readonly reload: Reload
	) {}

	/** Run a write; reload on success; say why on failure. Resolves whether it worked. */
	async #act(write: () => Promise<unknown>): Promise<boolean> {
		this.actionError = null;
		try {
			await write();
			await this.reload();
			return true;
		} catch (e) {
			this.actionError = message(e, 'that action could not be completed');
			return false;
		}
	}

	/** Admin — a member's dōjō role (the row carries the underlying user id). */
	async setRole(m: KitMember, role: string): Promise<void> {
		const userId = m.userId;
		if (userId) await this.#act(() => this.actions.setRole(userId, role));
	}

	/** Admin — remove a dōjō viewer, and their workspace views here (#307 D8). */
	removeViewer = (v: DojoViewerView) => this.#act(() => this.actions.removeViewer(v.principalId));
	/** Admin — grant / revoke a viewer's view of one of the dōjō's workspaces. */
	grantView = (v: DojoViewerView, workspaceId: string) => this.#act(() => this.actions.grantView(v.principalId, workspaceId));
	revokeView = (v: DojoViewerView, workspaceId: string) => this.#act(() => this.actions.revokeView(v.principalId, workspaceId));

	/** Admin — create (`teamId` null) or rename a team; resolves whether it saved. */
	saveTeam = (teamId: string | null, name: string) => this.#act(() => this.actions.saveTeam(teamId, name));
	deleteTeam = (t: TeamView) => this.#act(() => this.actions.deleteTeam(t.id));
	addTeamMember = (t: TeamView, principalId: string) => this.#act(() => this.actions.addTeamMember(t.id, principalId));
	removeTeamMember = (t: TeamView, principalId: string) =>
		this.#act(() => this.actions.removeTeamMember(t.id, principalId));
	/** Admin — grant everyone now on the team the view of a workspace. */
	grantTeamView = (t: TeamView, workspaceId: string) => this.#act(() => this.actions.grantTeamView(t.id, workspaceId));
}
