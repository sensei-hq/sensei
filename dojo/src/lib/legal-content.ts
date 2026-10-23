// The Terms of Service and Privacy Policy (#330). Each statement describes what
// the dōjō's code and schema actually do (database/ddl/table/dojo); the tests in
// legal-content.spec.ts pin the facts a reader relies on. The final legal
// wording is the owner's: this is a factual draft, reviewed before release.

export interface LegalSection {
	heading: string;
	body: string[];
}

export interface LegalPage {
	path: '/terms-of-service' | '/privacy-policy';
	title: string;
	/** ISO date of the last change to this page's text. */
	updated: string;
	sections: LegalSection[];
}

export const TERMS: LegalPage = {
	path: '/terms-of-service',
	title: 'Terms of Service',
	updated: '2026-10-23',
	sections: [
		{
			heading: 'What the dōjō is',
			body: [
				'The dōjō at dojo.sensei-hq.com is the shared side of sensei. sensei runs on your own machine; the dōjō is where you choose to share what it learns with a team: rules, patterns, a shared code graph of the repositories you choose to share, and aggregated metrics.',
				'Using the dōjō is optional. sensei works on its own without it.'
			]
		},
		{
			heading: 'Your account',
			body: [
				'You sign in with GitHub, with Microsoft, or with a link sent to your email. You are responsible for the account you sign in with and for what is done through it.',
				'Organization dōjōs are created from a GitHub organization or an Azure DevOps organization whose administrators connect it. Who can see what inside a dōjō follows the roles its administrators set.'
			]
		},
		{
			heading: 'Your content',
			body: [
				'You keep ownership of everything you or your organization contribute to the dōjō. You give us permission to store it and show it to the people your dōjō’s settings allow, only to run the service.',
				'Do not contribute content you have no right to share, or anything unlawful.'
			]
		},
		{
			heading: 'Acceptable use',
			body: [
				'Do not try to reach data you have not been given access to, disrupt the service, or use it to harm others. We may suspend access that does.'
			]
		},
		{
			heading: 'Connected services',
			body: [
				'When an administrator connects GitHub or Azure DevOps, the dōjō reads from them with the permissions granted and never writes to them. Your use of those services stays under their own terms.'
			]
		},
		{
			heading: 'Availability and changes',
			body: [
				'The service is provided as is, without warranties. We may change or discontinue features; when these terms change, the date above changes with them.'
			]
		},
		{
			heading: 'Leaving',
			body: [
				'You can stop using the dōjō at any time. When you leave an organization, or its administrators remove you, your membership ends. See the Privacy Policy for what happens to your data.'
			]
		},
		{
			heading: 'Contact',
			body: ['Questions about these terms: hi@sensei-hq.com.']
		}
	]
};

export const PRIVACY: LegalPage = {
	path: '/privacy-policy',
	title: 'Privacy Policy',
	updated: '2026-10-23',
	sections: [
		{
			heading: 'Who you are to the dōjō',
			body: [
				'To sign you in we keep your login (email address) and the identities you sign in with: your GitHub account id and login, or your Microsoft Entra object id. We use them to know who you are and which organizations you belong to.'
			]
		},
		{
			heading: 'What your organizations share',
			body: [
				'Memberships and roles: which dōjōs you belong to, as what, and how you joined.',
				'Repositories: their addresses and names, and which workspace they belong to.',
				'A shared code graph for the repositories a member chooses to share: file paths, content hashes, and symbol names, signatures, docstrings and descriptions, with their line ranges. This is structure only, never the contents of your files.',
				'Shared rules, patterns and other contributions, with who contributed them and the decisions made on them.',
				'Metrics as aggregates per repository and per person (for example sessions or tokens per day). Sessions, prompts and turns stay on your machine. Below the lead role, team totals are shown only when enough other people contributed.',
				'Relay: if you supervise a run from your phone, the run’s outline, summaries and the replies you send are kept so you can review them. Only you can see your runs.',
				'An audit log of administrative actions, such as who granted or removed access.'
			]
		},
		{
			heading: 'Credentials for connected services',
			body: [
				'When an administrator connects Azure DevOps, the refresh token Microsoft issues is kept encrypted in Supabase Vault and read only by the server; access tokens are never stored. A GitHub token from your sign-in is used during that request and, for the integrations screen, kept for at most an hour in an encrypted cookie in your browser.'
			]
		},
		{
			heading: 'Who processes your data',
			body: [
				'Supabase hosts the database and authentication. Cloudflare runs the dōjō’s web application. GitHub and Microsoft are asked only what an administrator or you have allowed, to sign you in and to read organization membership.'
			]
		},
		{
			heading: 'How long we keep it',
			body: [
				'Your data is kept while your account and your organizations use the dōjō. When you leave an organization, or a complete read of its members no longer lists you, your membership ends and you drop out of its team views; what you contributed stays with that organization. Removing a connection deletes its stored credential.'
			]
		},
		{
			heading: 'Your choices',
			body: [
				'You decide what sensei shares: nothing leaves your machine until you choose to share it. You can ask for your account and the data tied to it to be exported or deleted by writing to hi@sensei-hq.com.'
			]
		},
		{
			heading: 'Changes',
			body: [
				'When this policy changes, the date above changes with it.'
			]
		},
		{
			heading: 'Contact',
			body: ['Questions about your data, or a request to export or delete it: hi@sensei-hq.com.']
		}
	]
};

export const LEGAL_PAGES: LegalPage[] = [TERMS, PRIVACY];
