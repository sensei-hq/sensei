// The product catalog on the Plan & billing screen: two plans (D46). Free —
// every public repository, and a personal dōjō's private ones. Org — an
// organization's private repositories too, at one price per active contributor.
// Product copy, the same for every org — not any org's account. Which plan an
// org is on comes from its billing account (`billing-map`).
import type { KitBillingTier } from './components/kit/types';

/** The Org plan's monthly price per active contributor, in dollars. */
export const ORG_PRICE_PER_CONTRIBUTOR = 12;

export const BILLING_TIERS: KitBillingTier[] = [
	{
		id: 'free',
		kanji: '無',
		name: 'Free',
		price: '$0',
		sub: 'every organization',
		lines: [
			'Every public repository syncs',
			"A personal dōjō's private repositories sync too",
			'Unlimited members · full governance authoring',
			"An organization's private repositories stay hidden"
		]
	},
	{
		id: 'org',
		kanji: '組',
		name: 'Org',
		price: '$' + ORG_PRICE_PER_CONTRIBUTOR,
		sub: 'per active contributor / month',
		lines: [
			'Every private repository syncs, on by default',
			"Only the organization's admins can turn a repository's sync off",
			'Read-only members are always free',
			'Role consoles · audit · a shared Relay inbox'
		]
	}
];
