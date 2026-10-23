// One interface for every git provider a dōjō can come from (#308 R5a, D20/D37).
//
// A UNIT is what a provider groups repositories by — a GitHub org; an Azure
// DevOps organization and its projects (R5b); a GitLab group. An import reads the
// units a person can see, proves which of them they administer, and follows each
// unit's member list. Spec: docs/spec/dojo/git-integration-import.md.
//
// THE `complete` RULE. Every list read reports whether it saw everything. Only a
// complete read may END anything — a membership, a provisioned dōjō's member —
// because an org the read could not name must not look like an org the person
// left (#309). A read that fails outright THROWS; it is never spelled as an
// empty list, and `proveAdmin` never answers `false` for an outage.
import { AdminError } from './admin-data';
import { githubAdapter, type ForgeOrg, type ForgeProvider } from './forge-github';

/** One unit the person can see, and their standing in it — the forge's stable id
 *  (`tenant_connections.external_id`), its name (`external_slug`) and whether the
 *  forge calls them an admin there. The same shape provisioning reads orgs in. */
export type ForgeUnit = ForgeOrg;

/** A unit as an adapter is asked about it: by stable id, addressed by name. */
export type ForgeUnitRef = Pick<ForgeUnit, 'id' | 'login'>;

/** One member of a unit, as the forge names them. */
export interface ForgeMember {
	/** The forge's stable user id — `dojo.identities.subject`. */
	id: string;
	login: string;
}

/** The units one read found, and whether it saw every one. */
export interface ForgeUnitsRead {
	units: ForgeUnit[];
	complete: boolean;
}

/** The members one read found, and whether it saw every one. */
export interface ForgeMembersRead {
	members: ForgeMember[];
	complete: boolean;
}

/** Everything the dōjō asks of a provider, with the person's own token. */
export interface ForgeAdapter {
	provider: ForgeProvider;
	/** The units the token's owner is an active member of. */
	listUnits(token: string): Promise<ForgeUnitsRead>;
	/** Whether the token's owner administers this unit, as the provider says now.
	 *  `false` is a positive answer ("not an admin", "not a member"); a read that
	 *  could not be made throws. */
	proveAdmin(token: string, unit: ForgeUnitRef): Promise<boolean>;
	/** Every member of the unit the token can see. */
	listMembers(token: string, unit: ForgeUnitRef): Promise<ForgeMembersRead>;
}

/**
 * The adapter for one provider. `fetchImpl` is injected for tests.
 *
 * Only GitHub is answered here. Azure DevOps' adapter is bound to ONE named
 * organization (`azureAdapter(org)` in forge-azure.ts — the person picks it from
 * the organizations their Entra connect can see), so it is built by the Azure
 * routes, which have the organization; asked for by provider alone it is a 501, like GitLab and
 * Bitbucket. Answering any of them with the GitHub adapter would send one forge's
 * token to another's API and read the answer as truth.
 */
export function forgeAdapter(provider: ForgeProvider, fetchImpl: typeof fetch = fetch): ForgeAdapter {
	if (provider === 'github') return githubAdapter(fetchImpl);
	throw new AdminError(501, `importing from ${provider} is not available yet`);
}
