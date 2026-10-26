// One workspace's daemon-resolved constitution, as the preview drill-ins show it
// (the personal /you/workspaces/[id] and the org /org/[slug]/workspaces/[id]).
// The daemon owns resolution; the dōjō reads the federated result by workspace
// id and maps it to the kit ladder + conflicts. Nothing federated yet is empty;
// a failed read is an error the screen shows, never an empty constitution (#106).
import { getWorkspaceConstitution } from './client-data';
import { rulesToLadder, relayRules, relayToKitConflicts } from './constitution-map';
import type { DojoCallOpts } from './dojo-api';
import type { KitLadderRung, KitConflict } from './components/kit/types';

export interface WorkspaceConstitutionView {
	ladder: KitLadderRung[];
	conflicts: KitConflict[];
	error: string | null;
}

export async function loadWorkspaceConstitution(
	workspaceId: string,
	opts: DojoCallOpts = {}
): Promise<WorkspaceConstitutionView> {
	try {
		const c = await getWorkspaceConstitution(workspaceId, opts);
		if (!c) return { ladder: [], conflicts: [], error: null };
		return { ladder: rulesToLadder(relayRules(c)), conflicts: relayToKitConflicts(c), error: null };
	} catch (e) {
		return { ladder: [], conflicts: [], error: e instanceof Error ? e.message : 'the request failed' };
	}
}
