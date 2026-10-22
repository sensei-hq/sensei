// Admin → Integrations (#308 R5a) as the screen shows it: the wire shapes of
// POST /v1/t/{tenant}/integrations/{provider}[/claim|/map] (the server builds
// them in $lib/server/integrations-data.ts) and the words each row says. Pure.

import type { ForgeProvider } from './forge-providers';

/** A unit's own dōjō — the one its forge connection names. */
export interface IntegrationDojo {
	key: string;
	name: string;
	claimed: boolean;
	/** Who claimed it, by display name; null when unclaimed or unnamed. */
	claimedBy: string | null;
}

/** Where a unit's repositories file today, when someone has mapped it. */
export type IntegrationMapping =
	| { as: 'dojo' }
	| {
			as: 'workspace';
			/** Whether the workspace is in the dōjō whose screen this is. */
			inThisDojo: boolean;
			/** The dōjō's key and the workspace's name — null when the dōjō is not
			 *  one the person is in: another dōjō's contents are not theirs to read. */
			dojoKey: string | null;
			workspaceName: string | null;
	  };

/** One unit (a GitHub org) on the Integrations screen. */
export interface IntegrationUnit {
	provider: ForgeProvider;
	id: string;
	login: string;
	/** The person's standing in the unit, as the forge said on this read. */
	role: 'admin' | 'member';
	/** Its own dōjō, or null when none has been made. */
	dojo: IntegrationDojo | null;
	mapping: IntegrationMapping | null;
	/** An admin of the unit, and its dōjō unclaimed (or not made yet). */
	canClaim: boolean;
	/** An admin of the unit. */
	canMapAsDojo: boolean;
	/** An admin of the unit — or anyone, while it is unclaimed and no admin of it
	 *  has placed it. The database enforces the same rule (map_forge_unit). */
	canMapAsWorkspace: boolean;
}

/** POST …/integrations/{provider}. */
export interface IntegrationUnitsView {
	provider: ForgeProvider;
	units: IntegrationUnit[];
	/** Whether the forge read saw every unit. */
	complete: boolean;
}

/** Where a unit goes: its own dōjō, or a workspace of the screen's dōjō. */
export type MapTarget = { as: 'dojo' } | { as: 'workspace'; name?: string };

/** What the unit's own dōjō is — and, unclaimed, who can claim it. */
export function dojoLine(u: IntegrationUnit): string {
	if (!u.dojo) return 'No dōjō yet';
	if (!u.dojo.claimed) return `${u.dojo.key} · unclaimed — an admin of ${u.login} on GitHub can claim it`;
	return u.dojo.claimedBy ? `${u.dojo.key} · claimed by ${u.dojo.claimedBy}` : `${u.dojo.key} · claimed`;
}

/** Where the unit's repositories file. */
export function mappingLine(u: IntegrationUnit): string {
	const m = u.mapping;
	if (!m) return u.dojo ? 'Repositories file into its own dōjō' : 'Repositories file nowhere yet';
	if (m.as === 'dojo') return 'Repositories file into its own dōjō';
	if (m.inThisDojo && m.workspaceName) return `Repositories file into the workspace ${m.workspaceName} in this dōjō`;
	if (m.dojoKey && m.workspaceName) return `Repositories file into the workspace ${m.workspaceName} in ${m.dojoKey}`;
	return 'Repositories file into a workspace of another dōjō';
}
