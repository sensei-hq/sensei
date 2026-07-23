// The Your-work landing state (chunk 1 — mockup `ScrYourWork`).
//
// The needs-you band is a remote control: the viewer acts inline rather than
// being routed away. This module owns the small derivations the presentational
// screen needs — the urgency ordering, the per-kind lead action, and a rune-
// backed `resolved` map with the derived open-count / next-need / week stats.
// Kept as a `.svelte.ts` rune module (like `preview-state.svelte.ts`) so the
// `resolved` map is reactive `$state`; the pure helpers stay unit-testable
// without a component. Ported from dojo2-app.jsx (`K2_URGENCY` · `K2_PRIMARY` ·
// `orderNeeds` · the `onAct` reducer).

import type { KitNeed } from './components/kit/types';

/** Urgency rank for ordering the needs-you band (gate → conflict → decision →
 *  review). Lower = more urgent (mockup `K2_URGENCY`). */
export const K2_URGENCY: Record<string, number> = {
	gate: 0,
	conflict: 1,
	decision: 2,
	review: 3
};

/** The primary CTA label per need kind (mockup `K2_PRIMARY`). */
const K2_PRIMARY: Record<string, string> = {
	gate: 'Approve',
	conflict: 'Settle',
	decision: 'Decide',
	review: 'Approve'
};

/** The act id fired through the band's `onAct` for a kind's primary action. */
const K2_ACT: Record<string, string> = {
	gate: 'approve',
	conflict: 'settle',
	decision: 'decide',
	review: 'approve'
};

/** How an act id resolves a need (the label stored in the `resolved` map). */
const ACT_RESOLVED: Record<string, string> = {
	deny: 'denied',
	settle: 'settled',
	decide: 'decided',
	approve: 'approved'
};

/** Order needs by urgency (gate first). Pure — copies before sorting. */
export function orderNeeds(items: KitNeed[]): KitNeed[] {
	return items
		.slice()
		.sort((a, b) => (K2_URGENCY[a.kind] ?? 9) - (K2_URGENCY[b.kind] ?? 9));
}

/** The primary CTA label for a need kind, defaulting to Decide. */
export function primaryLabel(kind: string): string {
	return K2_PRIMARY[kind] ?? 'Decide';
}

/** The act id a need kind's primary action fires, defaulting to decide. */
export function actForKind(kind: string): string {
	return K2_ACT[kind] ?? 'decide';
}

/** The minimal project shape the week stat needs. */
export interface RunsWeekProject {
	runsWeek?: number;
}

/** The minimal run shape the active-run count needs. */
export interface RunState {
	state: string;
}

/** The stat inputs — the projects + runs the header badges summarize. */
export interface YourWorkStats {
	projects?: RunsWeekProject[];
	runs?: RunState[];
}

/** The reactive landing state: the ordered needs, the resolved map, and the
 *  derived open list / next need / week stats. */
export interface YourWorkState {
	/** The needs ordered by urgency. */
	readonly ordered: KitNeed[];
	/** `{ [needId]: label }` — reactive; grows as the viewer acts. */
	readonly resolved: Record<string, string>;
	/** The still-open needs (ordered). */
	readonly openItems: KitNeed[];
	/** The most-urgent open need, or `undefined` when all are resolved. */
	readonly next: KitNeed | undefined;
	/** Total sessions run this week across the projects. */
	readonly runsWeek: number;
	/** Currently-running sessions. */
	readonly activeRuns: number;
	/** Resolve a need via an act id (approve · deny · settle · decide). */
	act(item: KitNeed, actId: string): void;
}

/** Build the Your-work landing state from the needs (+ optional stat inputs).
 *  The `resolved` map is `$state` so the band reacts to inline acts. */
export function createYourWork(items: KitNeed[], stats: YourWorkStats = {}): YourWorkState {
	const ordered = orderNeeds(items);
	const resolved = $state<Record<string, string>>({});
	const runsWeek = (stats.projects ?? []).reduce((a, p) => a + (p.runsWeek ?? 0), 0);
	const activeRuns = (stats.runs ?? []).filter((r) => r.state === 'running').length;

	const openItems = $derived(ordered.filter((it) => !resolved[it.id]));

	return {
		ordered,
		get resolved() {
			return resolved;
		},
		get openItems() {
			return openItems;
		},
		get next() {
			return openItems[0];
		},
		runsWeek,
		activeRuns,
		act(item: KitNeed, actId: string) {
			resolved[item.id] = ACT_RESOLVED[actId] ?? 'approved';
		}
	};
}
