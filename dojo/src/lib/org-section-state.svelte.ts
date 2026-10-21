// The org-zone console writes (#293) — a member's role, incidents, engagements —
// and the incident detail pane. The page binds each write to the org's tenant
// and session; this runs it, reloads on success, and says why on failure.
import { DojoApiError } from '$lib/client-data';
import type { KitEngagement, KitIncident, KitIncidentDetail, KitMember } from '$lib/components/kit/types';

/** The console writes, already bound to the org's tenant and session token. */
export interface OrgActions {
	setRole: (userId: string, role: string) => Promise<unknown>;
	reportIncident: (title: string) => Promise<unknown>;
	resolveIncident: (id: string) => Promise<unknown>;
	removeIncident: (id: string) => Promise<unknown>;
	openIncident: (id: string) => Promise<KitIncidentDetail>;
	newEngagement: (clientName: string) => Promise<unknown>;
	closeEngagement: (id: string) => Promise<unknown>;
	removeEngagement: (id: string) => Promise<unknown>;
}

/** Re-read the section's load (the page passes invalidateAll). */
export type Reload = () => Promise<unknown>;

const message = (e: unknown, fallback: string) => (e instanceof DojoApiError ? e.message : fallback);

export class OrgSectionState {
	/** Why the last action failed — shown under the screen, never swallowed. */
	actionError = $state<string | null>(null);
	/** The incident whose detail pane is open. */
	incidentDetail = $state<KitIncidentDetail | null>(null);

	constructor(
		private readonly actions: OrgActions,
		private readonly reload: Reload
	) {}

	async #act(write: () => Promise<unknown>): Promise<void> {
		this.actionError = null;
		try {
			await write();
			await this.reload();
		} catch (e) {
			this.actionError = message(e, 'that action could not be completed');
		}
	}

	/** Admin — a member's dōjō role (the row carries the underlying user id). */
	setRole(m: KitMember, role: string): Promise<void> {
		const userId = m.userId;
		return userId ? this.#act(() => this.actions.setRole(userId, role)) : Promise.resolve();
	}

	reportIncident = (title: string) => this.#act(() => this.actions.reportIncident(title));

	/** Resolve an incident; an open pane on it is re-read so it shows the change. */
	resolveIncident = async (i: KitIncident) => {
		await this.#act(() => this.actions.resolveIncident(i.id));
		if (!this.actionError && this.incidentDetail?.id === i.id) await this.openIncident(i);
	};

	/** Delete an incident; an open pane on it closes. */
	removeIncident = async (i: KitIncident) => {
		await this.#act(() => this.actions.removeIncident(i.id));
		if (!this.actionError && this.incidentDetail?.id === i.id) this.closeIncident();
	};

	/** Read an incident's detail on demand; a failed read opens no pane. */
	async openIncident(i: KitIncident): Promise<void> {
		this.actionError = null;
		try {
			this.incidentDetail = await this.actions.openIncident(i.id);
		} catch (e) {
			this.actionError = message(e, 'could not open that incident');
		}
	}

	closeIncident(): void {
		this.incidentDetail = null;
	}

	newEngagement = (clientName: string) => this.#act(() => this.actions.newEngagement(clientName));
	closeEngagement = (e: KitEngagement) => this.#act(() => this.actions.closeEngagement(e.id));
	removeEngagement = (e: KitEngagement) => this.#act(() => this.actions.removeEngagement(e.id));
}
