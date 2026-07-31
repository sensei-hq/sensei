// Pure wire→kit mappers for the org admin console screens (dojo
// `/org/[slug]/members` · `/audit` · `/identity` · `/health`). Take the
// `admin-data.ts` wire types (the SAME rows the shipped `(console)` admin screens
// fetch) and project them onto the presentational Kit shapes the dojo screens
// already declare. Side-effect-free so they're DRY + unit-tested once; `now` is
// injected for a deterministic "since".
//
// On display gaps: the wire rows carry ids + status, not display names. A
// membership has a `user_id` (uuid), not a person's name — the name lives on the
// identity row. Where a display name isn't available on the row, the mapper falls
// back to a stable, honest label (a shortened id) rather than fabricating a name.

import type { Membership, Identity, Policy, AuditEvent, HealthRollup } from './admin-data';
import { relativeAge } from './triage/view';
import type {
	KitMember,
	KitRolePolicy,
	KitChatTurn,
	KitIdentity,
	KitIdentityMapping,
	KitHealth,
	KitHealthSignal,
	KitHealthWeek
} from './components/kit/types';

/** A short, stable display handle for a bare uuid (first 8 chars). */
function shortId(id: string | null | undefined): string {
	return id ? id.slice(0, 8) : '—';
}

/** The git-access line the role derived from (the wire carries the auth method,
 *  not a git-permission string, so this reads the method honestly). */
function accessLine(authenticatedVia: string): string {
	switch (authenticatedVia) {
		case 'github_oauth':
			return 'GitHub';
		case 'sso':
			return 'SSO';
		case 'device_code':
			return 'Device code';
		default:
			return authenticatedVia || '—';
	}
}

/**
 * Membership → KitMember. `name` is the display name resolved from the member's
 * identity row (WS-1: `listMembers` enriches each row via `resolveDisplayNames`),
 * falling back to a short user-id when the member has no named identity (honest
 * label, never a fabricated name); `git` is the authentication method the role
 * derived from; `scopes` is "—" (per-scope ownership is a separate read);
 * `active` is the relative last-heartbeat; a disabled member reads "disabled". Pure.
 */
export function toKitMember(
	m: Membership,
	opts: { self?: string; now?: Date } = {}
): KitMember {
	const now = opts.now ?? new Date();
	const active = m.disabled_at
		? 'disabled'
		: m.last_heartbeat_at
			? relativeAge(m.last_heartbeat_at, now)
			: 'never';
	const member: KitMember = {
		name: m.display_name ?? shortId(m.user_id),
		userId: m.user_id,
		git: accessLine(m.authenticated_via),
		role: m.role,
		scopes: '—',
		active
	};
	if (opts.self && m.user_id === opts.self) member.you = true;
	return member;
}

/** Membership[] → KitMember[], preserving the API's order. Pure. */
export function toKitMembers(members: Membership[], opts: { self?: string; now?: Date } = {}): KitMember[] {
	return members.map((m) => toKitMember(m, opts));
}

// ── policies → the additive role-policy grid ─────────────────────────────────
// The role-policy grid the mockup shows is the fixed additive ladder (developer
// → maintainer → lead → admin), independent of the tenant's per-scope
// `dojo.policies` rows (which are attribution/retention/confidentiality config,
// not the role ladder). The ladder is a constant so the screen renders it
// verbatim; the tenant policies feed the count/summary chips.

const ROLE_LADDER: readonly KitRolePolicy[] = [
	{ id: 'developer', kanji: '士', label: 'developer', note: 'read-mostly · watches & chats' },
	{ id: 'maintainer', kanji: '掟', label: 'maintainer', note: 'authors governance · triages' },
	{ id: 'lead', kanji: '客', label: 'lead', note: 'client engagements & audit' },
	{ id: 'admin', kanji: '任', label: 'admin', note: 'member roles & policies' }
];

/** The additive role-policy ladder (constant — the same rungs for every org).
 *  `_policies` is accepted so a caller can pass the tenant policy rows through a
 *  single wire→kit call site; the ladder itself is scope-independent. Pure. */
export function toKitRolePolicies(_policies: Policy[] = []): KitRolePolicy[] {
	return [...ROLE_LADDER];
}

// ── audit events → the audit-tab chat thread ─────────────────────────────────

/** A compact human line for an audit event: the action + its target. */
function auditText(e: AuditEvent): string {
	const target = e.target ? ` · ${e.target}` : '';
	return `${e.action}${target}`;
}

/**
 * AuditEvent → KitChatTurn. The Audit tab renders the governance log as a
 * sensei-voiced thread; each event becomes one turn (`who = 'sensei'`, the action
 * as the text, the relative age as the timestamp). Pure.
 */
export function toKitChatTurn(e: AuditEvent, now: Date = new Date()): KitChatTurn {
	return {
		who: 'sensei',
		kanji: '録',
		text: auditText(e),
		when: relativeAge(e.ts, now)
	};
}

/** AuditEvent[] → KitChatTurn[], preserving the API's (most-recent-first) order.
 *  Pure. */
export function toKitAuditThread(events: AuditEvent[], now: Date = new Date()): KitChatTurn[] {
	return events.map((e) => toKitChatTurn(e, now));
}

// ── identities → the Identity & SSO config ───────────────────────────────────

/** A human provider label for the IdP card / a mapping source. */
function providerLabel(provider: string): string {
	switch (provider) {
		case 'sso':
			return 'SSO';
		case 'github_oauth':
			return 'GitHub';
		case 'device_code':
			return 'Device code';
		default:
			return provider || 'Identity';
	}
}

/**
 * Identity[] → KitIdentity. There's no IdP config row on the wire (SCIM / IdP
 * connection is org settings, not an identity mapping), so the header reads the
 * dominant provider among the mapped identities and an honest "connected" when
 * any identity exists / "not connected" when none. The mappings are the
 * per-provider identity counts. Pure.
 */
export function toKitIdentity(identities: Identity[]): KitIdentity {
	const byProvider = new Map<string, number>();
	for (const id of identities) {
		byProvider.set(id.provider, (byProvider.get(id.provider) ?? 0) + 1);
	}
	// The dominant provider heads the IdP card.
	let topProvider = '';
	let topCount = 0;
	for (const [p, n] of byProvider) {
		if (n > topCount) {
			topProvider = p;
			topCount = n;
		}
	}
	const connected = identities.length > 0;
	const mappings: KitIdentityMapping[] = [...byProvider.entries()].map(([provider, count]) => ({
		source: providerLabel(provider),
		to: 'role from directory access',
		count
	}));
	return {
		idp: {
			name: connected ? providerLabel(topProvider) : 'No provider',
			protocol: topProvider === 'sso' ? 'SAML / OIDC' : 'OAuth',
			status: connected ? 'connected' : 'not connected',
			domain: connected ? `${identities.length} mapped` : '—'
		},
		scim: false,
		mappings
	};
}

// ── health rollup → the vital-signs grid ─────────────────────────────────────

/**
 * HealthRollup → KitHealth. Projects the four rollup counts onto the signal
 * cards, plus the real contributions-vs-approvals weekly series (from the health
 * route's `dojo.audit_events` bucketing; an empty series renders no bars). Anomaly
 * alerts still wait on the containment seam, so those render empty. Pure.
 */
export function toKitHealth(rollup: HealthRollup): KitHealth {
	const signals: KitHealthSignal[] = [
		{ kanji: '観', label: 'Live connections', n: String(rollup.connections), sub: 'last 5 min', tone: 'accent' },
		{ kanji: '門', label: 'Queue depth', n: String(rollup.queue_depth), sub: 'awaiting triage', tone: 'ink' },
		{
			kanji: '承',
			label: 'Published · 1h',
			n: String(rollup.publish_rate_1h),
			sub: 'last hour',
			tone: 'success'
		},
		{
			kanji: '盾',
			label: 'Sync errors',
			n: String(rollup.error_rate_1h),
			sub: rollup.error_rate_1h > 0 ? 'needs a look' : 'all healthy',
			tone: rollup.error_rate_1h > 0 ? 'warning' : 'success'
		}
	];
	const contribVsApprove: KitHealthWeek[] = (rollup.contrib_vs_approve ?? []).map((w) => ({
		wk: w.wk,
		c: w.c,
		a: w.a
	}));
	return { signals, contribVsApprove, alerts: [] };
}
