// The product catalog on the Plan & billing screen: two plans, as the approved
// mockup (coord/mockups/L4-billing-r3) draws them (D46, D52, D59, D75, D85, D86).
// Free — an organization's public repositories and its members' personal dōjōs,
// targets that warn everywhere, and the admin features that are free on every plan
// (SSO, SCIM, the audit trail, the relay's team queue). Pro — the organization's
// private repositories too, with blocking targets, integrations and consolidated
// metrics, billed per seat; the price is to be announced until a payment provider
// exists (D75), so no amount is shown. Product copy, the same for every org — which
// plan an org is on comes from its billing account (`billing-map`).
import type { KitBillingTier } from './components/kit/types';

export const BILLING_TIERS: KitBillingTier[] = [
	{
		id: 'free',
		kanji: '無',
		name: 'Free',
		price: 'No charge',
		sub: 'for every organization',
		lines: [
			"This organization's public repositories, on by default",
			"Members' personal dōjōs, private repositories and blocking targets included",
			'Targets that warn, on every repository',
			"Governance rules and standards sent to every member's sensei",
			'Admin control over which public repositories sync',
			'SSO, SCIM and the audit trail',
			"The relay's team on-call queue",
			'Unlimited members; viewers and contributors to public repositories are never counted'
		]
	},
	{
		id: 'pro',
		kanji: '組',
		name: 'Pro',
		price: 'To be announced',
		sub: 'per seat on private repositories',
		lines: [
			'Everything in Free',
			"This organization's private repositories, all on by default",
			'Blocking targets on those private repositories',
			'Integrations and consolidated metrics for those private repositories',
			'Admin control over which private repositories sync',
			'Options: extended audit retention and an SLA'
		]
	}
];
