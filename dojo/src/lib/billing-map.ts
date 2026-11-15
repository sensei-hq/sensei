// Map the live `/v1/…/billing` response onto the Plan & billing screen's
// KitBilling shape, as the approved mockup (coord/mockups/L4-billing-r3) draws it.
// Only what the billing plane knows is shown: the billable seats with their names
// (dojo.tenant_seat_usage + the identity join) and, when a billing account exists,
// its plan and state. With no account the org is on Free (D46). There is no price
// until a payment provider exists (D75), and no invoice (D-BILLING) — never a
// mockup's (#106).
//
// Each account state carries one notice with its next step, and agrees with
// dojo.all_my_repositories (M5): the status is checked before the period, so past
// due stays Pro (tagged) while its private repositories are hidden; canceled or a
// lapsed period is Free again. Pro that ran and ended says it has lapsed; a Pro
// plan that never ran (a future start, or never active) is not enabled (user
// ruling 2026-11-06). A plan key sensei does not know is said as an error, never
// shown as Free. Pure, so it unit-tests without a DOM.
import type { KitBilling, KitBillingProblem } from './components/kit/types';
import type { BillingResponse } from './admin-data';
import { BILLING_TIERS } from './billing-catalog';
import { countedSeats } from './billing-view';

/** ISO date → a short "Aug 1" renewal label; passthrough if unparseable.
 *  Formatted in UTC so the label matches the stored calendar day regardless of
 *  the viewer's timezone. */
function shortDate(iso: string): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

/** The catalog plan a raw plan key is on: "org" and "team" were Pro's earlier
 *  names (D75); `null` for a key sensei does not know. */
function planId(plan: string): 'free' | 'pro' | null {
	if (plan === 'free') return 'free';
	if (plan === 'pro' || plan === 'org' || plan === 'team') return 'pro';
	return null;
}

type PlanId = 'free' | 'pro';

const LAPSED = 'Pro has lapsed.';
const NOT_ENABLED = 'Pro is not enabled.';
const RENEW = 'Renew once payments are connected.';
const HIDDEN_AGAIN = "this organization's private repositories are hidden again.";

const tierName = (id: PlanId) => BILLING_TIERS.find((t) => t.id === id)?.name ?? null;

/** One state of the screen: which plan is current, its tag and notice. */
function screenFor(
	res: BillingResponse,
	id: PlanId | null,
	tag: KitBilling['tag'],
	problem: KitBillingProblem | null,
	renews: string | null = null
): KitBilling {
	return {
		plan: id === null ? null : tierName(id),
		seatsActive: res.usage.seats_used,
		seatsReadonly: null,
		renews,
		tiers: BILLING_TIERS.map((t) => ({ ...t, current: t.id === id })),
		invoices: [],
		problem,
		tag,
		badge: id === null ? 'Plan unknown' : tierName(id),
		showPlans: id !== null,
		seats: countedSeats(res.usage.billable_users)
	};
}

/** Why a Pro account with a current status is outside its period, or `null`.
 *  `lapsed` tells Pro that ran and ended from a period that has not started. */
function outsidePeriod(
	account: NonNullable<BillingResponse['account']>,
	now: Date
): { why: string; lapsed: boolean } | null {
	if (account.period_start && now < new Date(account.period_start))
		return { why: `The billing period starts ${shortDate(account.period_start)}`, lapsed: false };
	if (account.period_end && now > new Date(account.period_end))
		return { why: `The billing period ended ${shortDate(account.period_end)}`, lapsed: true };
	return null;
}

/** The lapsed notice (Pro ran and ended): Free again, and it says so plainly. */
function lapsed(why: string, code: string): KitBillingProblem {
	return { tone: 'refusal', title: 'Pro is not active', detail: `${why}, so ${HIDDEN_AGAIN} ${LAPSED}`, remedy: RENEW, code };
}

/** The not-enabled notice (Pro never ran: a future start, or never active). */
function notEnabled(why: string, code: string): KitBillingProblem {
	return { tone: 'refusal', title: 'Pro is not active', detail: `${why}, so ${HIDDEN_AGAIN} ${NOT_ENABLED}`, remedy: RENEW, code };
}

/** The screen built from the live response. Pure; `now` for the period check. */
export function toKitBilling(res: BillingResponse, now: Date = new Date()): KitBilling {
	const account = res.account;
	if (!account) return screenFor(res, 'free', 'Current plan', null);
	const id = planId(account.plan);
	if (id === null) {
		return screenFor(res, null, null, {
			tone: 'fault',
			title: "This organization's plan could not be read",
			detail: `The billing account says "${account.plan}", which isn't a plan sensei knows.`,
			remedy: 'Ask whoever set up billing to correct the plan key. Until then the plan is unknown.',
			code: 'unknown_plan'
		});
	}
	if (id === 'free') return screenFor(res, 'free', 'Current plan', null);
	// The status first, as all_my_repositories checks it: past due is still Pro.
	if (account.status === 'past_due') {
		return screenFor(res, 'pro', 'Past due', {
			tone: 'refusal',
			title: 'Pro payment is past due',
			detail: "This organization's private repositories are hidden until the payment is settled. They come back as they were once it is paid.",
			remedy: 'Settle the payment once payments are connected.',
			code: 'not_subscribed'
		});
	}
	if (account.status !== 'active' && account.status !== 'trialing') {
		const why = `The billing account is ${account.status.replace(/_/g, ' ')}`;
		// Canceled is Pro that ran and ended; any other status has never been active.
		return account.status === 'canceled'
			? screenFor(res, 'free', 'Current plan', lapsed(why, 'not_subscribed'))
			: screenFor(res, 'free', 'Current plan', notEnabled(why, 'not_enabled'));
	}
	const out = outsidePeriod(account, now);
	if (out) {
		return screenFor(
			res,
			'free',
			'Current plan',
			// An ended period or trial lapsed; a period that has not started is not enabled.
			out.lapsed ? lapsed(out.why, 'subscription_expired') : notEnabled(out.why, 'not_enabled')
		);
	}
	if (account.status === 'trialing') {
		// A trial ends; it never "renews" (L4).
		const ends = account.period_end ? ` — ends ${shortDate(account.period_end)}` : '';
		return screenFor(res, 'pro', 'Trial', {
			tone: 'normal',
			title: `Pro trial${ends}`,
			detail: 'Private repositories sync during the trial.',
			remedy: 'Nothing to do now. Plan changes open once payments are connected.',
			code: 'trialing'
		});
	}
	const renews = account.period_end ? shortDate(account.period_end) : null;
	return screenFor(
		res,
		'pro',
		'Current plan',
		{
			tone: 'normal',
			title: 'Pro is attached',
			detail: `Private repositories are on and sync at each member's next sync pass.${renews ? ` Renews ${renews}.` : ''}`,
			remedy: 'Nothing to do.',
			code: 'active'
		},
		renews
	);
}

/** The screen with nothing loaded (another section, or a failed read whose
 *  error the page shows): no plan, no tag, no seats, no plan cards — never Free. */
export function noBilling(): KitBilling {
	return {
		plan: null,
		seatsActive: null,
		seatsReadonly: null,
		renews: null,
		tiers: BILLING_TIERS,
		invoices: [],
		problem: null,
		tag: null,
		badge: null,
		showPlans: false,
		seats: []
	};
}
