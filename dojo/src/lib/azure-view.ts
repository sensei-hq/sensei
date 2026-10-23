// Azure DevOps on Admin → Integrations and in the personal dōjō (#308 R5b, D42)
// as the screen shows it: the wire shapes of the connect flow
// (/v1/t/{tenant}/integrations/azure_devops[/connect|/sync|/token],
// /v1/you/integrations/azure_devops/connect, /v1/integrations/azure_devops/
// {organizations,import} — the server builds them in $lib/server/azure-*.ts),
// the read scopes the Microsoft Entra connect asks for, and the words each line
// says. Pure, so a client module may import it.

/**
 * The Azure DevOps scopes the dōjō asks Microsoft Entra for — read only
 * (https://learn.microsoft.com/en-us/azure/devops/integrate/get-started/authentication/oauth#available-scopes).
 * `vso.profile` reads the organizations you belong to (Profiles and Accounts
 * APIs); the rest read one organization. The connect also asks for
 * `offline_access` (the refresh token the dōjō keeps) and `openid`.
 */
export const AZURE_REQUIRED_SCOPES = [
	{ scope: 'vso.profile', label: 'User profile (read)', why: 'list the organizations you belong to' },
	{ scope: 'vso.project', label: 'Project and team (read)', why: 'list an organization’s projects' },
	{ scope: 'vso.graph', label: 'Graph (read)', why: 'prove you are a Project Collection Administrator' },
	{ scope: 'vso.memberentitlementmanagement', label: 'Member entitlement management (read)', why: 'read who is in the organization' },
	{ scope: 'vso.code', label: 'Code (read)', why: 'read its repositories' }
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
	/** When the last complete member read replaced the member set; null: none yet. */
	memberSetReadAt: string | null;
	/** Older than dojo.member_set_max_age_hours (or never read): a Microsoft
	 *  sign-in joins nobody from it (#308 R5c review). */
	memberSetStale: boolean;
}

/** GET …/integrations/azure_devops. */
export interface AzureStatusView {
	connections: AzureConnectionView[];
	scopes: readonly AzureScope[];
	/** Whether this server has the app registration configured. */
	configured: boolean;
}

/** POST /v1/integrations/azure_devops/organizations — what the pending connect
 *  can import. */
export interface AzureGrantView {
	organizations: { id: string; name: string }[];
	/** Whether every organization could be read. */
	complete: boolean;
	/** Granted scopes beyond the read ones (the warning). */
	writeScopes: string[];
	/** Where an import goes: this organization dōjō, or its own dōjō. */
	target: 'this' | 'own';
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
 *  provisioned members it no longer lists and joined the listed people who had
 *  already signed in (#308 R5c); a partial one ended and joined nobody; a failed
 *  one is said, and ended nobody. */
export type AzureMembersRead = { complete: boolean; listed: number; ended: number; joined: number } | { error: string };

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

/** The stored grant's state, and its expiry when one is known. */
/** Whether the member list a Microsoft sign-in joins from is fresh (#308 R5c). */
export function memberSetLine(c: AzureConnectionView): string {
	if (!c.memberSetReadAt) return 'No complete member list yet — Microsoft sign-ins join nobody until a Sync reads one';
	const at = `Member list read ${c.memberSetReadAt.slice(0, 10)}`;
	return c.memberSetStale ? `${at} is stale — Microsoft sign-ins join nobody from it until a Sync reads it again` : at;
}

export function tokenLine(c: AzureConnectionView): string {
	if (c.token === 'none') return 'Not connected — connect Azure DevOps to sync';
	if (c.token === 'needs_new_token') return 'Microsoft no longer accepts this connection — reconnect Azure DevOps';
	return c.tokenExpiresAt ? `Connected through Microsoft Entra · expires ${c.tokenExpiresAt.slice(0, 10)}` : 'Connected through Microsoft Entra';
}

const CONNECT_ERRORS: Record<string, string> = {
	denied: 'The Microsoft sign-in was cancelled or consent was declined',
	expired: 'The connect expired or was started in another session — connect again',
	state: 'The connect could not be verified — connect again',
	entra: 'Microsoft would not complete the connect — try again, or ask an Entra admin to allow the app',
	signin: 'Sign in to the dōjō first, then connect Azure DevOps',
	not_configured: 'Azure DevOps is not configured on this dōjō',
	too_many: 'Too many attempts — try again in a few minutes'
};

/** What the connect came to, from the callback's redirect query — or null. Only
 *  known codes are said; anything else in the URL is ignored. */
export function connectNotice(q: URLSearchParams): { ok: boolean; text: string } | null {
	if (q.get('azure') === 'connected') return { ok: true, text: 'Connected to Azure DevOps — pick the organizations to import' };
	const code = q.get('azure_error');
	return code && Object.hasOwn(CONNECT_ERRORS, code) ? { ok: false, text: CONNECT_ERRORS[code] } : null;
}

/** The write-scope warning, or null when the consent carries read scopes only. */
export function writeScopesLine(scopes: string[]): string | null {
	if (!scopes.length) return null;
	return `This Microsoft consent also grants ${scopes.join(', ')}. The dōjō only reads — a Microsoft Entra admin can remove these from the app’s permissions.`;
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
	if (m.joined) parts.push(`${m.joined} who had signed in now ${m.joined === 1 ? 'a member' : 'members'}`);
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

/** Whether the login's Microsoft account is linked to the dōjō (#308 R5c):
 *  `GET /v1/you/microsoft`. Not linked means no identity was recorded and no
 *  dōjō joined from it, and says why. */
export type MicrosoftLinkStatus = { state: 'none' } | { state: 'linked' } | { state: 'not_linked'; reason: string };

/** What My dōjōs says about it — only when it could not be linked. */
export function microsoftNotice(s: MicrosoftLinkStatus): string | null {
	if (s.state !== 'not_linked') return null;
	return `Couldn’t link your Microsoft account: ${s.reason}. Your Azure DevOps organizations’ dōjōs were not joined.`;
}
