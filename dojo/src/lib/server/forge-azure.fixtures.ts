// Recorded-shape answers from the Azure DevOps REST API, for forge-azure.spec.ts.
// Test-only. Each shape follows the Microsoft doc (or client contract) cited on
// the reader in forge-azure.ts; the names and ids are made up.

export const ORG = 'example-org';
export const ORG_ID = '10feb381-82c3-4902-8e1f-840299a48ae4';
export const ME_ID = 'd6245f20-2af8-44f4-9451-8107cb2767db';
/** The token owner's Microsoft Entra object id (GraphUser.originId, origin aad). */
export const ME_OID = '55c8c7b6-7ace-43bc-918f-304dfa2b6317';
export const ME_DESCRIPTOR = 'aad.OWI3MWYyMTYtNGM0Zi03Yjc0LWE5MTEtZWZiMGZhOWM3Nzdm';
export const PCA_DESCRIPTOR = 'vssgp.Uy0xLTktMTU1MTM3NDI0NS0yMTc2MDU2ODQ4LTAtMC0wLTAtMQ';
export const NESTED_GROUP = 'vssgp.Uy0xLTktMTU1MTM3NDI0NS1uZXN0ZWQtYWRtaW5z';
export const OTHER_GROUP = 'vssgp.Uy0xLTktMTU1MTM3NDI0NS1jb250cmlidXRvcnM';

/** GET https://dev.azure.com/{org}/_apis/connectionData */
export const connectionData = (over: Record<string, unknown> = {}, user: Record<string, unknown> = {}) => ({
	authenticatedUser: {
		id: ME_ID,
		descriptor: 'Microsoft.IdentityModel.Claims.ClaimsIdentity;example\\admin@example.com',
		subjectDescriptor: ME_DESCRIPTOR,
		providerDisplayName: 'Example Admin',
		isActive: true,
		...user
	},
	authorizedUser: { id: ME_ID, subjectDescriptor: ME_DESCRIPTOR, providerDisplayName: 'Example Admin' },
	instanceId: ORG_ID,
	deploymentId: '00000000-0000-4000-8000-000000000001',
	deploymentType: 'hosted',
	locationServiceData: {},
	...over
});

const group = (descriptor: string, principalName: string, domain = `vstfs:///Framework/IdentityDomain/${ORG_ID}`) => ({
	subjectKind: 'group',
	domain,
	principalName,
	origin: 'vsts',
	originId: '00326ae2-d77d-4803-95af-398944a07df9',
	displayName: principalName.split('\\')[1],
	descriptor
});

/** GET https://vssps.dev.azure.com/{org}/_apis/graph/groups — page 1 (no PCA) */
export const groupsPage1 = {
	count: 2,
	value: [
		group('vssgp.enterprise', '[TEAM FOUNDATION]\\Enterprise Service Accounts', 'vstfs:///Framework/IdentityDomain/9b4f7336-c130-40bd-87f3-4201361ddbd1'),
		group(OTHER_GROUP, `[${ORG}]\\Project Collection Valid Users`)
	]
};

/** …page 2 — carries the organization's Project Collection Administrators */
export const groupsPage2 = {
	count: 1,
	value: [group(PCA_DESCRIPTOR, `[${ORG}]\\Project Collection Administrators`)]
};

/** GET https://vssps.dev.azure.com/{org}/_apis/graph/Memberships/{descriptor}?direction=up */
export const memberships = (member: string, containers: string[]) => ({
	count: containers.length,
	value: containers.map((c) => ({ containerDescriptor: c, memberDescriptor: member }))
});

/** GET https://vssps.dev.azure.com/{org}/_apis/graph/users/{descriptor} */
export const graphUser = (over: Record<string, unknown> = {}) => ({
	subjectKind: 'user',
	domain: '45aa3d2d-7442-473d-b4d3-3c670da9dd96',
	principalName: 'admin@example.com',
	mailAddress: 'admin@example.com',
	origin: 'aad',
	originId: ME_OID,
	displayName: 'Example Admin',
	descriptor: ME_DESCRIPTOR,
	...over
});

const entitlement = (id: string | undefined, principalName: string, status = 'active', originId?: string, origin = 'aad') => ({
	id,
	user: {
		subjectKind: 'user',
		principalName,
		mailAddress: principalName,
		displayName: principalName.split('@')[0],
		origin,
		originId,
		descriptor: 'aad.ZTk3YjBlN2YtMGE2MS03MWFkLTg2MGMtNzQ4ZWM1ZmNiMjBi'
	},
	accessLevel: { licensingSource: 'account', accountLicenseType: 'express', status, licenseDisplayName: 'Basic' },
	lastAccessedDate: '2026-10-01T10:00:00Z',
	dateCreated: '2026-01-01T10:00:00Z'
});

/** GET https://vsaex.dev.azure.com/{org}/_apis/userentitlements — page 1 */
export const entitlementsPage1 = {
	members: [
		entitlement(ME_ID, 'admin@example.com', 'active', ME_OID),
		entitlement('7c5a4b1e-0000-4000-8000-000000000002', 'dev@example.com', 'active', 'a0e10000-0000-4000-8000-000000000002')
	],
	continuationToken: 'page-2',
	totalCount: 4
};

/** …page 2, the last (no continuation token) — one removed user */
export const entitlementsPage2 = {
	members: [
		entitlement('7c5a4b1e-0000-4000-8000-000000000003', 'ops@example.com', 'active', 'a0e10000-0000-4000-8000-000000000003'),
		entitlement('7c5a4b1e-0000-4000-8000-000000000004', 'gone@example.com', 'deleted', 'a0e10000-0000-4000-8000-000000000004'),
		// A GitHub-backed Azure DevOps user carries no Entra object id: no dōjō
		// identity can match it (D43), so it is neither listed nor a gap.
		entitlement('7c5a4b1e-0000-4000-8000-000000000005', 'octo-dev', 'active', '1234567', 'github')
	],
	continuationToken: null,
	totalCount: 4
};

/** GET https://dev.azure.com/{org}/_apis/projects */
export const projects = (n: number, from = 0) => ({
	count: n,
	value: Array.from({ length: n }, (_, i) => ({
		id: `6ce954b1-ce1f-45d1-b94d-${String(from + i).padStart(12, '0')}`,
		name: `Project ${from + i}`,
		url: `https://dev.azure.com/${ORG}/_apis/projects/x`,
		state: 'wellFormed',
		visibility: 'private'
	}))
});
