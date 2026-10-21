// The dōjō's viewers (#307 D8) as the Viewers tab shows them. Pure: the wire rows
// from GET …/viewers in, the view rows out.

export interface ViewerProjectView {
	id: string;
	name: string;
}

export interface DojoViewerView {
	principalId: string;
	name: string | null;
	email: string | null;
	projects: ViewerProjectView[];
}

/** One row of GET /v1/t/{tenant}/viewers. */
export interface DojoViewerWire {
	principal_id: string;
	display_name: string | null;
	email: string | null;
	joined_at: string;
	projects: ViewerProjectView[];
}

export function toViewerViews(rows: DojoViewerWire[]): DojoViewerView[] {
	return rows.map((r) => ({ principalId: r.principal_id, name: r.display_name, email: r.email, projects: r.projects }));
}

/** A viewer's label: their name, else their email, else a short id — never an invented name. */
export function viewerLabel(v: DojoViewerView): string {
	return v.name ?? v.email ?? v.principalId.slice(0, 8);
}
