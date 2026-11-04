// Pure presentation helpers for the admin Plan & billing console (ScrBilling).
// Side-effect-free (billing state in → actions and copy out) so the plan
// actions and the attach notice unit-test without a DOM and the screen stays
// declarative.

import type { KitBilling } from './components/kit/types';

// ── wire (GET /v1/t/{tenant}/billing) — the one declaration; the server's
// billing-data and the admin client both import it ─────────────────────────

/** A billable user and the private workspaces seating them. */
export interface BillableUserWire {
	user_id: string;
	workspaces: { name: string; slug: string; role: string }[];
}

/** Live seat usage for a tenant (`dojo.tenant_seat_usage`). */
export interface SeatUsageWire {
	/** Unique users with ≥1 active seat on a private workspace — the billable count. */
	seats_used: number;
	/** All active seat rows considered (private + public), for transparency. */
	total_active_seats: number;
	/** Per-user breakdown of the billable seats (private workspaces only), sorted. */
	billable_users: BillableUserWire[];
}

/** A tenant's billing account snapshot (`dojo.billing_accounts`). */
export interface BillingAccountWire {
	plan: string;
	status: string;
	seats_included: number;
	seats_used: number;
	seats_computed_at: string | null;
	period_start: string | null;
	period_end: string | null;
}

/** A plan card's button: none on the current plan, or when the current plan is
 *  not known; on Free, Pro offers to attach (the primary step); on Pro, Free
 *  offers to move back. */
export interface TierAction {
	label: string;
	primary: boolean;
}

export function tierAction(id: string, current: string | null): TierAction | null {
	if (current === null || id === current) return null;
	return id === 'pro' ? { label: 'Attach Pro', primary: true } : { label: 'Move to Free', primary: false };
}

/** The id of the current plan, or `null` when the account's plan is unknown. */
export function currentTier(b: Pick<KitBilling, 'tiers'>): string | null {
	return b.tiers.find((t) => t.current)?.id ?? null;
}

/** Why the plan buttons cannot run before a payment provider is connected
 *  (D-BILLING): shown whenever one is offered (M6). A Notice (normal tone) —
 *  waiting, not failing. */
export function attachNotice(current: string) {
	return {
		tone: 'normal' as const,
		title: 'Plan changes open once payments are connected',
		detail:
			current === 'free'
				? 'Until then this organization stays on Free: its public repositories sync, and its private ones stay hidden.'
				: 'Until then this plan stays as it is.',
		code: 'billing_not_connected'
	};
}
