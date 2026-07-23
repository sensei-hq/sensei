// Per-kind vocab for the needs-you band (ported from dojo2-kit.jsx
// K2_NEEDS_TONE / K2_NEEDS_ACTIONS). The band is a remote control: each kind of
// blocked item leads with its own icon and offers its own action set, so the
// viewer acts here rather than being routed away. Kept token-native and pure so
// the maps unit-test and the components stay presentational.

/** The leading icon + verb for a needs-kind. */
export interface NeedsTone {
	/** Solar icon name for the row's leading mark. */
	icon: string;
	/** The one-word verb this kind asks of the viewer. */
	label: string;
	/** Token text class for the icon tint. */
	toneClass: string;
}

/** gate · conflict · decision · review → its icon + verb + tint. */
export const NEEDS_TONE: Record<string, NeedsTone> = {
	gate: { icon: 'command', label: 'approve', toneClass: 'text-accent' },
	conflict: { icon: 'danger-triangle', label: 'settle', toneClass: 'text-warning' },
	decision: { icon: 'checklist-minimalistic', label: 'decide', toneClass: 'text-accent' },
	review: { icon: 'clipboard-check', label: 'review', toneClass: 'text-ink-soft' }
};

/** One button in a needs-kind's action set. */
export interface NeedsAction {
	/** Stable id passed back through `onAct` (approve · deny · settle · decide). */
	id: string;
	label: string;
	icon: string;
	/** Renders as the primary CTA; the rest are ghost buttons. */
	primary?: boolean;
}

/** gate → approve/deny · conflict → settle · decision → decide · review → open/deny. */
export const NEEDS_ACTIONS: Record<string, NeedsAction[]> = {
	gate: [
		{ id: 'approve', label: 'Approve', icon: 'check-circle', primary: true },
		{ id: 'deny', label: 'Deny', icon: 'close-circle' }
	],
	conflict: [{ id: 'settle', label: 'Settle', icon: 'scale', primary: true }],
	decision: [{ id: 'decide', label: 'Decide', icon: 'checklist-minimalistic', primary: true }],
	review: [
		{ id: 'approve', label: 'Approve', icon: 'check-circle', primary: true },
		{ id: 'deny', label: 'Decline', icon: 'close-circle' }
	]
};

/** Look up a needs-kind tone, defaulting to `decision`. */
export function needsTone(kind: string | null | undefined): NeedsTone {
	return (kind && NEEDS_TONE[kind]) || NEEDS_TONE.decision;
}

/** Look up a needs-kind action set, defaulting to `decision`. */
export function needsActions(kind: string | null | undefined): NeedsAction[] {
	return (kind && NEEDS_ACTIONS[kind]) || NEEDS_ACTIONS.decision;
}
