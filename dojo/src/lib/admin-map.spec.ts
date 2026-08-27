// The dojo org admin console wire→kit mappers (members / audit / identity /
// health). Deterministic `now`.
import { describe, expect, it } from 'vitest';
import type { Membership, Identity, AuditEvent, HealthRollup } from './admin-data';
import {
	toKitMember,
	toKitMembers,
	toKitRolePolicies,
	toKitChatTurn,
	toKitAuditThread,
	toKitIdentity,
	toKitHealth
} from './admin-map';

const NOW = new Date('2026-07-23T12:00:00Z');

function member(over: Partial<Membership> = {}): Membership {
	return {
		id: 'm1',
		user_id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
		role: 'maintainer',
		kind: 'employer',
		authenticated_via: 'github_oauth',
		sync_status: 'healthy',
		attribution_default: 'named',
		last_heartbeat_at: '2026-07-23T11:00:00Z',
		disabled_at: null,
		created_at: '2026-01-01T00:00:00Z',
		display_name: null,
		email: null,
		...over
	};
}

describe('toKitMember / toKitMembers', () => {
	it('maps the row, name falls back to a short user id', () => {
		const k = toKitMember(member(), { now: NOW });
		expect(k.name).toBe('aaaaaaaa');
		expect(k.userId).toBe('aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee');
		expect(k.git).toBe('GitHub');
		expect(k.role).toBe('maintainer');
		expect(k.scopes).toBe('—');
		expect(k.active).toBe('1h');
		expect(k.you).toBeUndefined();
	});
	it('uses the resolved display name when present (WS-1)', () => {
		expect(toKitMember(member({ display_name: 'Ada Lovelace' }), { now: NOW }).name).toBe('Ada Lovelace');
	});
	it('marks the viewer row with you', () => {
		const k = toKitMember(member(), { self: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee', now: NOW });
		expect(k.you).toBe(true);
	});
	it('reads disabled / never-active honestly', () => {
		expect(toKitMember(member({ disabled_at: '2026-07-01T00:00:00Z' }), { now: NOW }).active).toBe('disabled');
		expect(toKitMember(member({ last_heartbeat_at: null }), { now: NOW }).active).toBe('never');
	});
	it('maps a list preserving order', () => {
		const out = toKitMembers([member({ user_id: '11111111-x' }), member({ user_id: '22222222-y' })], { now: NOW });
		expect(out.map((m) => m.name)).toEqual(['11111111', '22222222']);
	});
});

describe('toKitRolePolicies', () => {
	it('returns the additive ladder regardless of tenant policies', () => {
		const ladder = toKitRolePolicies([]);
		expect(ladder.map((r) => r.id)).toEqual(['developer', 'maintainer', 'lead', 'admin']);
	});
});

function ev(over: Partial<AuditEvent> = {}): AuditEvent {
	return {
		id: 1,
		ts: '2026-07-23T11:22:00Z',
		actor_id: 'u1',
		engagement_id: null,
		action: 'publish',
		target: 'rule-9',
		detail: {},
		...over
	};
}

describe('toKitChatTurn / toKitAuditThread', () => {
	it('maps an event to a sensei-voiced turn', () => {
		const t = toKitChatTurn(ev(), NOW);
		expect(t.who).toBe('sensei');
		expect(t.text).toBe('publish · rule-9');
		expect(t.when).toBe('38m');
	});
	it('omits target dot when no target', () => {
		expect(toKitChatTurn(ev({ target: null }), NOW).text).toBe('publish');
	});
	it('maps a thread preserving order', () => {
		expect(toKitAuditThread([ev({ id: 1 }), ev({ id: 2 })], NOW)).toHaveLength(2);
	});
});

function identity(over: Partial<Identity> = {}): Identity {
	return {
		id: 'id1',
		principal_id: 'p1',
		provider: 'github_oauth',
		subject: 'gh|1',
		email: null,
		display_name: null,
		created_at: '2026-01-01T00:00:00Z',
		last_login_at: null,
		...over
	};
}

describe('toKitIdentity', () => {
	it('heads the IdP card with the dominant provider and counts mappings', () => {
		const k = toKitIdentity([
			identity({ provider: 'github_oauth' }),
			identity({ provider: 'github_oauth' }),
			identity({ provider: 'sso' })
		]);
		expect(k.idp.name).toBe('GitHub');
		expect(k.idp.status).toBe('connected');
		const gh = k.mappings.find((m) => m.source === 'GitHub');
		expect(gh?.count).toBe(2);
	});
	it('reads not-connected on no identities', () => {
		const k = toKitIdentity([]);
		expect(k.idp.status).toBe('not connected');
		expect(k.mappings).toEqual([]);
	});
});

describe('toKitHealth', () => {
	it('projects the rollup onto four signal cards', () => {
		const rollup: HealthRollup = { connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 0 };
		const h = toKitHealth(rollup);
		expect(h.signals).toHaveLength(4);
		expect(h.signals[0].n).toBe('3');
		expect(h.signals[1].n).toBe('12');
		expect(h.signals[3].tone).toBe('success'); // 0 errors → healthy
		expect(h.contribVsApprove).toEqual([]); // absent series → empty (no bars)
		expect(h.alerts).toEqual([]);
	});
	it('maps the contributions-vs-approvals weekly series when present', () => {
		const h = toKitHealth({
			connections: 0,
			queue_depth: 0,
			publish_rate_1h: 0,
			error_rate_1h: 0,
			contrib_vs_approve: [
				{ wk: 'W3', c: 4, a: 2 },
				{ wk: 'W4', c: 6, a: 5 }
			]
		});
		expect(h.contribVsApprove).toEqual([
			{ wk: 'W3', c: 4, a: 2 },
			{ wk: 'W4', c: 6, a: 5 }
		]);
	});
	it('flags sync errors as a warning tone', () => {
		const h = toKitHealth({ connections: 0, queue_depth: 0, publish_rate_1h: 0, error_rate_1h: 2 });
		expect(h.signals[3].tone).toBe('warning');
		expect(h.signals[3].sub).toMatch(/needs a look/);
	});
});
