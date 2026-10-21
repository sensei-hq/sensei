// A dōjō's teams (#307 D7) as the Teams tab shows them. Pure: the wire rows from
// GET …/teams in, the view rows out.
import type { KitMember } from '$lib/components/kit/types';
import type { PersonView } from '$lib/viewers-view';

export interface TeamView {
	id: string;
	name: string;
	members: PersonView[];
}

/** One row of GET /v1/t/{tenant}/teams. */
export interface TeamWire {
	id: string;
	name: string;
	members: Array<{ principal_id: string; display_name: string | null; email: string | null }>;
}

export function toTeamViews(rows: TeamWire[]): TeamView[] {
	return rows.map((t) => ({
		id: t.id,
		name: t.name,
		members: t.members.map((m) => ({ principalId: m.principal_id, name: m.display_name, email: m.email }))
	}));
}

/** The dōjō's members (with an id the API knows) not yet on the team. */
export function addableMembers(team: TeamView, members: KitMember[]): Array<{ id: string; name: string }> {
	const on = new Set(team.members.map((m) => m.principalId));
	return members.flatMap((m) => (m.userId && !on.has(m.userId) ? [{ id: m.userId, name: m.name }] : []));
}
