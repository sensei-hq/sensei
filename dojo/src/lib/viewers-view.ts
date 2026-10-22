// The dōjō's viewers (#307 D8) as the Viewers tab shows them. Pure: the wire rows
// from GET …/viewers in, the view rows out.

export interface ViewerWorkspaceView {
	id: string;
	name: string;
}

export interface DojoViewerView extends PersonView {
	workspaces: ViewerWorkspaceView[];
}

/** One row of GET /v1/t/{tenant}/viewers. */
export interface DojoViewerWire {
	principal_id: string;
	display_name: string | null;
	email: string | null;
	joined_at: string;
	workspaces: ViewerWorkspaceView[];
}

export function toViewerViews(rows: DojoViewerWire[]): DojoViewerView[] {
	return rows.map((r) => ({ principalId: r.principal_id, name: r.display_name, email: r.email, workspaces: r.workspaces }));
}

/** Someone the admin surfaces name: a viewer, a team member. */
export interface PersonView {
	principalId: string;
	name: string | null;
	email: string | null;
}

/** A person's label: their name, else their email, else a short id — never an invented name. */
export function personLabel(p: PersonView): string {
	return p.name ?? p.email ?? p.principalId.slice(0, 8);
}
