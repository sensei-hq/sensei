// Pure presentation helpers for the admin Plan & billing console (ScrBilling,
// mockup). Side-effect-free (billing state / free flag in → number or token
// classes out) so the monthly-total math and the free-vs-paid tone unit-test
// without a DOM and the screen stays declarative. The mockup stored raw
// `var(--*)` strings; here each tone is a named-token utility CLASS, never a raw
// oklch.

import type { KitBilling } from './components/kit/types';

/** The live monthly total — active (billable) contributors × the per-seat price
 *  (mockup `b.seatsActive * b.perSeat`). */
export function monthlyTotal(b: Pick<KitBilling, 'seatsActive' | 'perSeat'>): number {
	return b.seatsActive * b.perSeat;
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
