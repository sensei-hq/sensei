// The product catalog on the Plan & billing screen: the tiers and what Relay
// costs where. Product copy, the same for every org — not any org's account.
// Which tier an org is on comes from its billing account (`billing-map`).
import type { KitBillingTier, KitBillingRelayRow } from './components/kit/types';

export const BILLING_TIERS: KitBillingTier[] = [
	{
		id: 'free',
		kanji: '無',
		name: 'Free',
		price: 'Free',
		sub: 'public · OSS · personal',
		lines: [
			'Public / open-source or personal solo dōjō',
			'Unlimited members · full governance authoring',
			'Relay for your own workspaces — watch · approve · decide · chat',
			'Fair use: 1 active machine · standard realtime'
		]
	},
	{
		id: 'team',
		kanji: '組',
		name: 'Team',
		price: 'Per seat',
		sub: '/ mo · active contributor',
		lines: [
			'Private, shared scopes for a company or team',
			'Role consoles · audit',
			'Relay across the team — shared inbox, presence, priority realtime',
			'Read-only members always free'
		]
	},
	{
		id: 'enterprise',
		kanji: '企',
		name: 'Enterprise',
		price: 'Contract',
		sub: 'custom',
		dark: true,
		lines: [
			'Self-hosted / VPC · SSO (OIDC / SAML) + SCIM',
			'Audit retention & export · air-gapped bundle',
			'Self-hosted relay · SSO on mobile',
			'SLA & priority support'
		]
	}
];

export const RELAY_ROWS: KitBillingRelayRow[] = [
	{ label: 'Relay on your own workspaces — watch · approve · decide · chat', free: true },
	{ label: 'One active machine · standard realtime · native push', free: true },
	{ label: "Shared team inbox & queue · presence (who's handling this)", free: false },
	{ label: 'Higher concurrency · priority realtime · approval audit trail', free: false }
];
