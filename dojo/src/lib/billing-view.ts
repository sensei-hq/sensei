// Pure presentation helpers for the admin Plan & billing console (ScrBilling).
// Side-effect-free (billing state in → numbers, actions and copy out) so the
// estimate math, the plan actions and the attach notice unit-test without a DOM
// and the screen stays declarative.

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

/** The live monthly total — active (billable) contributors × the per-seat price;
 *  null when either is unknown (no price is set before a payment provider). */
export function monthlyTotal(b: Pick<KitBilling, 'seatsActive' | 'perSeat'>): number | null {
	return b.seatsActive === null || b.perSeat === null ? null : b.seatsActive * b.perSeat;
}

/** A plan card's button: none on the current plan; on Free, Org offers to
 *  attach (the primary step); on Org, Free offers to move back. */
export interface TierAction {
	label: string;
	primary: boolean;
}

export function tierAction(id: string, current: string): TierAction | null {
	if (id === current) return null;
	return id === 'org' ? { label: 'Attach Org plan', primary: true } : { label: 'Move to Free', primary: false };
}

/** What the screen says about changing plan before a payment provider is
 *  connected (D-BILLING): the actions are shown, but nothing can be attached or
 *  charged yet. A Notice (normal tone) — waiting, not failing. */
export function attachNotice() {
	return {
		tone: 'normal' as const,
		title: 'Plan changes open once payments are connected',
		detail: 'Until then every organization is on Free: its public repositories sync, and its private ones stay hidden.',
		code: 'billing_not_connected'
	};
}
