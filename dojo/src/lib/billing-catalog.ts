// The product catalog on the Plan & billing screen: two plans (D46, D52, D59,
// D75). Free — every public repository and a personal dōjō's private ones, with
// targets. Pro — an organization's private repositories too, with targets, billed
// by the seats who work on them. The price is to be announced until a payment
// provider exists (D75); no amount is shown. SSO, SCIM, audit, viewers and the
// relay's team on-call queue are free on every plan (D59, D77). Product copy, the
// same for every org — which plan an org is on comes from its billing account
// (`billing-map`).
import type { KitBillingTier } from './components/kit/types';

export const BILLING_TIERS: KitBillingTier[] = [
	{
		id: 'free',
		kanji: '無',
		name: 'Free',
		price: 'Free',
		sub: 'every organization',
		lines: [
			'Every public repository syncs, with targets',
			"A personal dōjō's private repositories sync too, with targets",
			'SSO, SCIM, audit, viewers and the relay’s team queue',
			"An organization's private repositories stay hidden"
		]
	},
	{
		id: 'pro',
		kanji: '組',
		name: 'Pro',
		price: 'To be announced',
		sub: 'per seat on private repositories',
		lines: [
			"Targets on an organization's private repositories",
			'Every private repository syncs, on by default',
			"Only the organization's admins can turn a repository's sync off",
			'Contributors to public repositories and viewers are never counted'
		]
	}
];
