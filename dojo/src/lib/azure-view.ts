// Azure DevOps on Admin → Integrations and in the personal dōjō (#308 R5b) as
// the screen shows it: the wire shapes of /v1/t/{tenant}/integrations/azure_devops
// [/connect|/sync|/token] and /v1/you/integrations/azure_devops/connect (the
// server builds them in $lib/server/azure-import.ts), the read-only scopes a PAT
// needs, and the words each line says. Pure, so a client module may import it.

/**
 * The scopes the personal access token needs — read only. Azure DevOps does not
 * let a PAT read its own scopes without a Microsoft Entra token (the PAT
 * Lifecycle Management API requires one), so the dōjō cannot check for write
 * scopes; the screen asks for exactly these.
 */
export const AZURE_REQUIRED_SCOPES = [
	{ scope: 'vso.project', label: 'Project and Team (Read)', why: 'list the organization’s projects' },
	{ scope: 'vso.graph', label: 'Graph (Read)', why: 'prove you are a Project Collection Administrator' },
	{ scope: 'vso.memberentitlementmanagement', label: 'Member Entitlement Management (Read)', why: 'read who is in the organization' },
	{ scope: 'vso.code', label: 'Code (Read)', why: 'read its repositories' }
] as const;

export type AzureScope = (typeof AZURE_REQUIRED_SCOPES)[number];

/** One of a dōjō's Azure DevOps organizations. */
export interface AzureConnectionView {
	/** The organization's stable id. */
	id: string;
	name: string;
	verifiedAt: string | null;
	/** The stored token: working, rejected by Azure, or none stored. */
	token: 'ok' | 'needs_new_token' | 'none';
	tokenExpiresAt: string | null;
}

/** GET …/integrations/azure_devops. */
export interface AzureStatusView {
	connections: AzureConnectionView[];
	scopes: readonly AzureScope[];
}

/** How an organization's projects filed. */
export interface AzureProjectsFiled {
	/** Projects the token could read. */
	total: number;
	/** Filed as workspaces of the dōjō. */
	mapped: number;
	/** Already filed into another dōjō by an admin there — left as they are. */
	elsewhere: number;
	/** Whether the project read saw every project. */
	complete: boolean;
}

/** What the member read came to (#309's rule): a complete read ended the
 *  provisioned members it no longer lists; a partial one ended nobody; a failed
 *  one is said, and ended nobody. */
export type AzureMembersRead = { complete: boolean; listed: number; ended: number } | { error: string };

/** What an import came to. Never carries the token. */
export interface AzureImportResult {
	dojo: { key: string; created: boolean };
	organization: { id: string; name: string };
	projects: AzureProjectsFiled;
	members: AzureMembersRead;
}

/** What a sync came to. */
export interface AzureSyncResult {
	organization: { id: string; name: string };
	projects: AzureProjectsFiled;
	members: AzureMembersRead;
}

/** The stored token's state, and its expiry when Azure reported one. */
export function tokenLine(c: AzureConnectionView): string {
	if (c.token === 'none') return 'No token stored — paste one to sync';
	if (c.token === 'needs_new_token') return 'Azure DevOps no longer accepts the stored token — paste a new one';
	return c.tokenExpiresAt
		? `Token stored · expires ${c.tokenExpiresAt.slice(0, 10)}`
		: 'Token stored · expiry not reported by Azure DevOps';
}

function filedParts(p: AzureProjectsFiled): string[] {
	const parts = [
		p.mapped === 0
			? 'no projects to file'
			: `${p.mapped} ${p.mapped === 1 ? 'project' : 'projects'} filed as ${p.mapped === 1 ? 'a workspace' : 'workspaces'}`
	];
	if (p.elsewhere) parts.push(`${p.elsewhere} already filed in another dōjō`);
	if (!p.complete) parts.push('Azure DevOps could not list every project');
	return parts;
}

function memberParts(m: AzureMembersRead): string[] {
	if ('error' in m) return [m.error];
	if (!m.complete) return ['the member list was incomplete, so nobody was ended'];
	const parts = [`${m.listed} ${m.listed === 1 ? 'member' : 'members'} read`];
	if (m.ended) parts.push(`${m.ended} who left the organization no longer ${m.ended === 1 ? 'a member' : 'members'}`);
	return parts;
}

/** Where an imported organization went, how its projects filed, and its members. */
export function importLine(r: AzureImportResult): string {
	const where = r.dojo.created
		? `${r.organization.name} is now the dōjō ${r.dojo.key}`
		: `${r.organization.name} is connected to ${r.dojo.key}`;
	return [where, ...filedParts(r.projects), ...memberParts(r.members)].join(' · ');
}

/** What a sync filed, and its members. */
export function syncLine(r: AzureSyncResult): string {
	return [`${r.organization.name} synced`, ...filedParts(r.projects), ...memberParts(r.members)].join(' · ');
}
