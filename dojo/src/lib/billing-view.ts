// Pure presentation helpers for the admin Plan & billing console (ScrBilling,
// mockup). Side-effect-free (billing state / free flag in → number or token
// classes out) so the monthly-total math and the free-vs-paid tone unit-test
// without a DOM and the screen stays declarative. The mockup stored raw
// `var(--*)` strings; here each tone is a named-token utility CLASS, never a raw
// oklch.

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

/** The token tone for a relay free-vs-paid chip. */
export interface RelayTone {
	text: string;
	soft: string;
	edge: string;
	label: string;
}

const RELAY_FREE: RelayTone = {
	text: 'text-success',
	soft: 'bg-success-soft',
	edge: 'border-success-soft',
	label: 'free · individuals'
};

const RELAY_PAID: RelayTone = {
	text: 'text-accent',
	soft: 'bg-accent-soft',
	edge: 'border-accent-soft',
	label: 'paid · team'
};

/** The chip tone + label for a relay row (mockup ternary): free rows read as
 *  success ("free · individuals"), paid rows as accent ("paid · team"). */
export function relayTone(free: boolean): RelayTone {
	return free ? RELAY_FREE : RELAY_PAID;
}

/** The label for a non-current tier's CTA (mockup: free ⇒ "Downgrade", else
 *  "Contact sales"). The current tier shows no CTA (a "current" chip instead). */
export function tierCtaLabel(id: string): string {
	return id === 'free' ? 'Downgrade' : 'Contact sales';
}
