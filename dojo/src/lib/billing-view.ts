// Pure presentation helpers for the admin Plan & billing console (ScrBilling).
// Side-effect-free (billing state in → actions and copy out) so the plan
// actions and the attach notice unit-test without a DOM and the screen stays
// declarative.

import type { KitBilling, KitBillingSeat } from './components/kit/types';
import { personLabel } from './viewers-view';

// ── wire (GET /v1/t/{tenant}/billing) — the one declaration; the server's
// billing-data and the admin client both import it ─────────────────────────

/** A billable user and the private workspaces seating them. */
export interface SeatHolderWire {
	user_id: string;
	workspaces: { name: string; slug: string; role: string }[];
}

/** A billable user as the route answers it: named through the identity join
 *  (`resolveDisplayNames`, as the system — the access-matrix exception). `null`
 *  name and email when the person shared neither; never an invented label. */
export interface BillableUserWire extends SeatHolderWire {
	display_name: string | null;
	email: string | null;
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

/** The seat count before names (`summarizeSeatUsage`): seat-holders only. */
export type SeatCountWire = Omit<SeatUsageWire, 'billable_users'> & { billable_users: SeatHolderWire[] };

/** The seat usage as the route answers it: each seat-holder with the name and
 *  email the identity join found. An id the join left unnamed is `null` — the
 *  screen says so in words (`personLabel`); nothing here invents a name. Pure. */
export function nameSeatHolders(
	count: SeatCountWire,
	names: ReadonlyMap<string, { display_name: string | null; email: string | null }>
): SeatUsageWire {
	return {
		...count,
		billable_users: count.billable_users.map((u) => ({
			...u,
			display_name: names.get(u.user_id)?.display_name ?? null,
			email: names.get(u.user_id)?.email ?? null
		}))
	};
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

/** "Who is counted": each seat as the admin surfaces name a person
 *  (`personLabel`: name, then email, then words — never an id fragment), with its
 *  workspaces and the role on each. */
export function countedSeats(users: BillableUserWire[]): KitBillingSeat[] {
	return users.map((u) => ({
		name: personLabel({ principalId: u.user_id, name: u.display_name, email: u.email }),
		workspaces: u.workspaces.map((w) => `${w.name} (${w.role})`).join(', ')
	}));
}

/** The seat section's copy: what the dōjō counts today, and the count Pro is
 *  priced by, marked proposed (D59). It says only what is built — a seat opens on
 *  a relay session; closing one is not built yet. */
export const SEAT_COPY = {
	label: 'active relay seats on private workspaces',
	basis: 'What the dōjō counts today: people with an active relay session on a private workspace.',
	remedy: "A seat opens when someone starts a relay session on a private workspace. Closing a seat isn't built yet.",
	proposed: { label: 'people working on private repositories', note: 'The count Pro is priced by (D59). Proposed — not in code.' }
} as const;

/** The hidden-repositories section's count line (user ruling 2026-11-05): hidden
 *  for everyone, the admin included. */
export function hiddenCountLine(count: number): string {
	return `${count} private ${count === 1 ? 'repository is' : 'repositories are'} hidden from everyone in this organization.`;
}

/** How the org's hidden private repositories come back, for the account's state:
 *  past due — once it is paid; a lapsed or canceled Pro — once it is renewed (both
 *  "as they were", D88); never on Pro — they sync once Pro is attached. */
export function hiddenReturnLine(b: Pick<KitBilling, 'tag' | 'problem'>): string {
	if (b.tag === 'Past due') return 'They come back as they were once it is paid.';
	if (b.problem?.code === 'not_subscribed' || b.problem?.code === 'subscription_expired') return 'They come back as they were once Pro is renewed.';
	return 'They sync once Pro is attached.';
}
