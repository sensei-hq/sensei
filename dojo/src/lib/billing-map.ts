// Map the live `/v1/…/billing` response onto the Plan & billing screen's
// KitBilling shape. Only what the billing plane knows is shown: the billable seat
// count (dojo.tenant_seat_usage) and, when a billing account exists, its plan and
// renewal date. With no account the org is on Free (D46). There is no price until
// a payment provider exists (D75), and no read-only count or invoice (D-BILLING) —
// never a mockup's (#106).
//
// An account counts only while it is current — status active or trialing and
// inside its period, the rule dojo.all_my_repositories applies — so this screen
// and the repositories it governs never disagree (M5). A plan key sensei does not
// know is said as an error, never shown as Free. Pure, so it unit-tests without a DOM.
import type { KitBilling, KitBillingProblem } from './components/kit/types';
import type { BillingResponse } from './admin-data';
import { BILLING_TIERS } from './billing-catalog';

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

const CURRENT = new Set(['active', 'trialing']);
const HIDDEN = "this organization's private repositories stay hidden.";

/** Why a paid account is not current, or `null` when it is (all_my_repositories' rule). */
function lapse(account: NonNullable<BillingResponse['account']>, now: Date): string | null {
	if (!CURRENT.has(account.status)) return `The billing account is ${account.status.replace(/_/g, ' ')}, so ${HIDDEN}`;
	const start = account.period_start ? new Date(account.period_start) : null;
	const end = account.period_end ? new Date(account.period_end) : null;
	if (start && now < start) return `The billing period starts ${shortDate(account.period_start!)}, so until then ${HIDDEN}`;
	if (end && now > end) return `The billing period ended ${shortDate(account.period_end!)}, so ${HIDDEN}`;
	return null;
}

/** The screen built from the live response: the seat count always, the plan,
 *  its renewal and the current plan from the account — Free without one, or
 *  when a paid account has lapsed. Pure; `now` for the period check. */
export function toKitBilling(res: BillingResponse, now: Date = new Date()): KitBilling {
	const account = res.account;
	let id: 'free' | 'pro' | null = account ? planId(account.plan) : 'free';
	let problem: KitBillingProblem | null = null;
	if (account && id === null) {
		problem = {
			tone: 'fault',
			title: "This dōjō's plan isn't one sensei knows",
			detail: `The billing account says "${account.plan}".`,
			remedy: 'Update Sensei. If it still shows, the billing account needs fixing.'
		};
	} else if (account && id === 'pro') {
		const why = lapse(account, now);
		if (why) {
			id = 'free';
			problem = { tone: 'refusal', title: 'The Pro plan is not active', detail: why, remedy: 'Plan changes open once payments are connected.' };
		}
	}
	const current = id !== null && !(problem && problem.tone === 'refusal');
	return {
		plan: id === null ? null : (BILLING_TIERS.find((t) => t.id === id)?.name ?? null),
		seatsActive: res.usage.seats_used,
		seatsReadonly: null,
		renews: current && account?.period_end ? shortDate(account.period_end) : null,
		tiers: BILLING_TIERS.map((t) => ({ ...t, current: t.id === id })),
		invoices: [],
		problem
	};
}

/** The screen with nothing loaded (another section, or a failed read whose
 *  error the page shows): no plan, no seats, no invoices. */
export function noBilling(): KitBilling {
	return {
		plan: null,
		seatsActive: null,
		seatsReadonly: null,
		renews: null,
		tiers: BILLING_TIERS,
		invoices: [],
		problem: null
	};
}
