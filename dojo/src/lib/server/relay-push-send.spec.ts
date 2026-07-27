// Unit tests for the relay P4.4 Web Push SEND logic (pure + injectable). The
// crypto/network send is impure and injected, so these tests need NO real VAPID
// keys or push service — they exercise:
//   - prefs gating: shouldPush (event opt-in, muted_tenants, quiet_hours incl. the
//     wrap-past-midnight case), eventForSignal, inQuietHours
//   - endpoint dedup: dedupByEndpoint (two rows for one endpoint → one push)
//   - the zero-knowledge payload builder (asserts NO code/command fields)
//   - the VAPID header shaping (RFC 8292 `vapid t=…, k=…`, aes128gcm)
//   - the send flow: gating short-circuit, dedup, 404/410 → disable (expired)
import { describe, it, expect, vi } from 'vitest';
import {
	eventForSignal,
	inQuietHours,
	shouldPush,
	dedupByEndpoint,
	buildRelayPushPayload,
	normalizeHeaders,
	sendRelayPush,
	type NotificationPrefs,
	type PushSignal,
	type StoredSubscription,
	type PushDb,
	type PushSender,
	type VapidKeys
} from './relay-push-send';

const APPROVAL: PushSignal = { type: 'inbox', kind: 'approval' };
const DECISION: PushSignal = { type: 'inbox', kind: 'decision' };
const STALL: PushSignal = { type: 'inbox', kind: 'stall' };
const CHAT: PushSignal = { type: 'inbox', kind: 'chat' };
const NUDGE: PushSignal = { type: 'inbox', kind: 'nudge' };
const CRASHED: PushSignal = { type: 'crashed' };

const T = 'tenant-1';

// A prefs row opted into everything, no mutes, no quiet-hours.
const OPTED_IN: NotificationPrefs = {
	events: { approvals: true, decisions: true, stalls: true, crashed: true },
	quiet_hours: {},
	muted_tenants: []
};

describe('eventForSignal', () => {
	it('maps needs-you kinds to their prefs event', () => {
		expect(eventForSignal(APPROVAL)).toBe('approvals');
		expect(eventForSignal(DECISION)).toBe('decisions');
		expect(eventForSignal(STALL)).toBe('stalls');
		expect(eventForSignal(CRASHED)).toBe('crashed');
	});
	it('returns null for chat / nudge (never push)', () => {
		expect(eventForSignal(CHAT)).toBeNull();
		expect(eventForSignal(NUDGE)).toBeNull();
	});
});

describe('inQuietHours', () => {
	// Fixed instants in UTC.
	const at = (hhmm: string) => new Date(`2026-07-17T${hhmm}:00.000Z`);

	it('non-wrapping window (09:00→17:00): inside vs outside', () => {
		const q = { tz: 'UTC', start: '09:00', end: '17:00' };
		expect(inQuietHours(q, at('12:00'))).toBe(true);
		expect(inQuietHours(q, at('09:00'))).toBe(true); // inclusive start
		expect(inQuietHours(q, at('17:00'))).toBe(false); // exclusive end
		expect(inQuietHours(q, at('08:59'))).toBe(false);
		expect(inQuietHours(q, at('23:00'))).toBe(false);
	});

	it('wrapping-past-midnight window (22:00→07:00)', () => {
		const q = { tz: 'UTC', start: '22:00', end: '07:00' };
		expect(inQuietHours(q, at('23:30'))).toBe(true); // late night
		expect(inQuietHours(q, at('02:00'))).toBe(true); // small hours
		expect(inQuietHours(q, at('06:59'))).toBe(true); // just before end
		expect(inQuietHours(q, at('07:00'))).toBe(false); // exclusive end
		expect(inQuietHours(q, at('12:00'))).toBe(false); // midday
		expect(inQuietHours(q, at('21:59'))).toBe(false); // just before start
	});

	it('respects the tz (a UTC instant maps to a different local hour)', () => {
		// 03:00 UTC = 23:00 the previous day in America/New_York (UTC-4 in July).
		const q = { tz: 'America/New_York', start: '22:00', end: '07:00' };
		expect(inQuietHours(q, at('03:00'))).toBe(true); // 23:00 NY → inside
		// 16:00 UTC = 12:00 NY → outside the night window.
		expect(inQuietHours(q, at('16:00'))).toBe(false);
	});

	it('absent / malformed / zero-width window never suppresses', () => {
		expect(inQuietHours({}, at('03:00'))).toBe(false);
		expect(inQuietHours(null, at('03:00'))).toBe(false);
		expect(inQuietHours({ start: 'nope', end: '07:00' }, at('03:00'))).toBe(false);
		expect(inQuietHours({ start: '22:00', end: '22:00' }, at('22:00'))).toBe(false);
	});
});

describe('shouldPush', () => {
	const now = new Date('2026-07-17T12:00:00.000Z');

	it('pushes an opted-in needs-you kind', () => {
		expect(shouldPush(APPROVAL, OPTED_IN, T, now)).toBe(true);
		expect(shouldPush(DECISION, OPTED_IN, T, now)).toBe(true);
		expect(shouldPush(STALL, OPTED_IN, T, now)).toBe(true);
		expect(shouldPush(CRASHED, OPTED_IN, T, now)).toBe(true);
	});

	it('never pushes chat / nudge even when a row exists', () => {
		expect(shouldPush(CHAT, OPTED_IN, T, now)).toBe(false);
		expect(shouldPush(NUDGE, OPTED_IN, T, now)).toBe(false);
	});

	it('is default-OFF: no prefs row, or the event not explicitly true', () => {
		expect(shouldPush(APPROVAL, null, T, now)).toBe(false);
		expect(shouldPush(APPROVAL, { events: {} }, T, now)).toBe(false);
		expect(shouldPush(APPROVAL, { events: { approvals: false } }, T, now)).toBe(false);
		// A different event opted in doesn't enable this one.
		expect(shouldPush(APPROVAL, { events: { stalls: true } }, T, now)).toBe(false);
	});

	it('suppresses a muted tenant', () => {
		const prefs: NotificationPrefs = { ...OPTED_IN, muted_tenants: [T] };
		expect(shouldPush(APPROVAL, prefs, T, now)).toBe(false);
		// A different tenant is unaffected.
		expect(shouldPush(APPROVAL, prefs, 'other', now)).toBe(true);
	});

	it('suppresses inside quiet-hours (wrap-past-midnight)', () => {
		const prefs: NotificationPrefs = {
			...OPTED_IN,
			quiet_hours: { tz: 'UTC', start: '22:00', end: '07:00' }
		};
		const night = new Date('2026-07-17T23:30:00.000Z');
		const day = new Date('2026-07-17T12:00:00.000Z');
		expect(shouldPush(APPROVAL, prefs, T, night)).toBe(false);
		expect(shouldPush(APPROVAL, prefs, T, day)).toBe(true);
	});
});

describe('dedupByEndpoint', () => {
	const sub = (id: string, endpoint: string): StoredSubscription => ({
		id,
		web_push: { endpoint, p256dh: 'PK', auth: 'AK' }
	});

	it('collapses two rows for one endpoint to a single (first) entry', () => {
		const out = dedupByEndpoint([sub('a', 'https://push/1'), sub('b', 'https://push/1'), sub('c', 'https://push/2')]);
		expect(out.map((s) => s.id)).toEqual(['a', 'c']);
	});

	it('drops rows missing endpoint / p256dh / auth', () => {
		const bad: StoredSubscription[] = [
			{ id: 'x', web_push: null },
			{ id: 'y', web_push: { endpoint: '', p256dh: 'PK', auth: 'AK' } },
			{ id: 'z', web_push: { endpoint: 'https://push/3', p256dh: '', auth: 'AK' } },
			sub('ok', 'https://push/4')
		];
		expect(dedupByEndpoint(bad).map((s) => s.id)).toEqual(['ok']);
	});
});

describe('buildRelayPushPayload (zero-knowledge)', () => {
	it('builds { title, body, url, tag } with the run title + deep link', () => {
		const p = buildRelayPushPayload(APPROVAL, 'Round-trip', 'run-42');
		expect(p.title).toBe('Dōjō Relay');
		expect(p.body).toBe('needs you on Round-trip');
		expect(p.url).toBe('/you/runs/run-42');
		expect(p.tag).toBe('/you/runs/run-42');
	});

	it('labels each signal type', () => {
		expect(buildRelayPushPayload(DECISION, 'R', 'r').body).toBe('needs a decision on R');
		expect(buildRelayPushPayload(STALL, 'R', 'r').body).toBe('stalled on R');
		expect(buildRelayPushPayload(CRASHED, 'R', 'r').body).toBe('crashed on R');
	});

	it('falls back to "your run" when the title is blank', () => {
		expect(buildRelayPushPayload(APPROVAL, '   ', 'r').body).toBe('needs you on your run');
		expect(buildRelayPushPayload(APPROVAL, null, 'r').body).toBe('needs you on your run');
	});

	it('carries ONLY logical fields — no code / command / diff keys', () => {
		const p = buildRelayPushPayload(APPROVAL, 'R', 'r') as Record<string, unknown>;
		expect(Object.keys(p).sort()).toEqual(['body', 'tag', 'title', 'url']);
		for (const forbidden of ['code', 'diff', 'command', 'payload', 'tool', 'reason', 'args', 'prompt']) {
			expect(p).not.toHaveProperty(forbidden);
		}
		// Whole payload serialised contains none of the forbidden substrings.
		const json = JSON.stringify(p);
		expect(json).not.toMatch(/cargo|rm -rf|diff|\.env|password/i);
	});
});

describe('normalizeHeaders', () => {
	it('rewrites VAPID auth to the RFC 8292 `vapid t=<jwt>, k=<key>` form', () => {
		const built = {
			'content-encoding': 'aesgcm',
			'content-type': 'application/octet-stream',
			'content-length': '135',
			ttl: '43200',
			authorization: 'WebPush the.jwt.token',
			'crypto-key': 'dh=SERVERDH;p256ecdsa=PUBKEY',
			encryption: 'salt=SALT',
			urgency: 'high'
		};
		const h = normalizeHeaders(built, 'PUBKEY');
		expect(h['Authorization']).toBe('vapid t=the.jwt.token, k=PUBKEY');
		expect(h['TTL']).toBe('43200');
		expect(h['Content-Type']).toBe('application/octet-stream');
		expect(h['Urgency']).toBe('high');
		expect(h['Content-Length']).toBe('135');
	});

	it('passes the aesgcm encoding + its companion headers through verbatim (header ↔ body match)', () => {
		const built = {
			'content-encoding': 'aesgcm',
			ttl: '43200',
			authorization: 'WebPush jwt',
			'crypto-key': 'dh=SERVERDH;p256ecdsa=PUBKEY',
			encryption: 'salt=SALT'
		};
		const h = normalizeHeaders(built, 'PUBKEY');
		// The encoding is NOT relabelled — it stays whatever the library encrypted with.
		expect(h['Content-Encoding']).toBe('aesgcm');
		// aesgcm REQUIRES the salt (Encryption) + server DH key (Crypto-Key) to decrypt.
		expect(h['Encryption']).toBe('salt=SALT');
		expect(h['Crypto-Key']).toBe('dh=SERVERDH;p256ecdsa=PUBKEY');
	});
});

// A chainable stub over the supabase-js query builder the send uses. Each
// `.from(table)` starts a fresh capture; awaiting the builder resolves the queued
// result for that table.
function makeDb(results: Record<string, { data: unknown; error: unknown }>) {
	const updates: { id?: unknown; patch?: unknown }[] = [];
	function builderFor(table: string) {
		const capture: { patch?: unknown; eqId?: unknown; op?: string } = {};
		const b: Record<string, unknown> = {};
		b.select = () => b;
		b.update = (patch: unknown) => {
			capture.op = 'update';
			capture.patch = patch;
			return b;
		};
		b.eq = (col: string, val: unknown) => {
			if (col === 'id') capture.eqId = val;
			return b;
		};
		// thenable → resolves the queued result; records an update if this was one.
		b.then = (resolve: (v: unknown) => void) => {
			if (capture.op === 'update') updates.push({ id: capture.eqId, patch: capture.patch });
			resolve(results[table] ?? { data: null, error: null });
		};
		return b as unknown;
	}
	const db: PushDb = { from: (t: string) => builderFor(t) as never };
	return { db, updates };
}

const VAPID: VapidKeys = { subject: 'mailto:x@y.z', publicKey: 'PUB', privateKey: 'PRIV' };

describe('sendRelayPush (flow)', () => {
	it('short-circuits (gated) when prefs opt-out — never loads subs or sends', async () => {
		const { db } = makeDb({
			notification_prefs: { data: [{ events: {} }], error: null }
		});
		const sender = vi.fn<PushSender>();
		const res = await sendRelayPush(
			{ db, vapid: VAPID, sender, now: new Date('2026-07-17T12:00:00Z') },
			{ userId: 'u', tenantId: T, runId: 'r', runTitle: 'R', signal: APPROVAL }
		);
		expect(res.gated).toBe(true);
		expect(res.attempted).toBe(0);
		expect(sender).not.toHaveBeenCalled();
	});

	it('sends one push per deduped subscription (2 rows, 1 endpoint → 1 send)', async () => {
		const { db } = makeDb({
			notification_prefs: { data: [OPTED_IN], error: null },
			push_subscriptions: {
				data: [
					{ id: 's1', web_push: { endpoint: 'https://push/1', p256dh: 'PK', auth: 'AK' } },
					{ id: 's2', web_push: { endpoint: 'https://push/1', p256dh: 'PK', auth: 'AK' } },
					{ id: 's3', web_push: { endpoint: 'https://push/2', p256dh: 'PK', auth: 'AK' } }
				],
				error: null
			}
		});
		const sender = vi.fn<PushSender>().mockResolvedValue(201);
		const res = await sendRelayPush(
			{ db, vapid: VAPID, sender, now: new Date('2026-07-17T12:00:00Z') },
			{ userId: 'u', tenantId: T, runId: 'r', runTitle: 'R', signal: APPROVAL }
		);
		expect(res.gated).toBe(false);
		expect(res.attempted).toBe(2); // deduped from 3 → 2 endpoints
		expect(res.delivered).toBe(2);
		expect(sender).toHaveBeenCalledTimes(2);
		// The sender received the zero-knowledge payload.
		const [, payload] = sender.mock.calls[0];
		expect(payload).toMatchObject({ title: 'Dōjō Relay', body: 'needs you on R', url: '/you/runs/r' });
	});

	it('disables a subscription on 404/410 (expired endpoint)', async () => {
		const { db, updates } = makeDb({
			notification_prefs: { data: [OPTED_IN], error: null },
			push_subscriptions: {
				data: [
					{ id: 'gone', web_push: { endpoint: 'https://push/gone', p256dh: 'PK', auth: 'AK' } },
					{ id: 'ok', web_push: { endpoint: 'https://push/ok', p256dh: 'PK', auth: 'AK' } }
				],
				error: null
			},
			// update() results (disable) resolve to no error.
		});
		const sender = vi.fn<PushSender>().mockResolvedValueOnce(410).mockResolvedValueOnce(201);
		const res = await sendRelayPush(
			{ db, vapid: VAPID, sender, now: new Date('2026-07-17T12:00:00Z') },
			{ userId: 'u', tenantId: T, runId: 'r', runTitle: 'R', signal: STALL }
		);
		expect(res.attempted).toBe(2);
		expect(res.delivered).toBe(1);
		expect(res.disabled).toEqual(['gone']);
		// The expired row was flipped enabled=false.
		expect(updates).toContainEqual(expect.objectContaining({ id: 'gone' }));
		const disabledPatch = updates.find((u) => u.id === 'gone')?.patch as Record<string, unknown>;
		expect(disabledPatch.enabled).toBe(false);
	});

	it('fail-open: a thrown sender does not sink the batch or throw', async () => {
		const { db } = makeDb({
			notification_prefs: { data: [OPTED_IN], error: null },
			push_subscriptions: {
				data: [
					{ id: 'boom', web_push: { endpoint: 'https://push/boom', p256dh: 'PK', auth: 'AK' } },
					{ id: 'ok', web_push: { endpoint: 'https://push/ok', p256dh: 'PK', auth: 'AK' } }
				],
				error: null
			}
		});
		const sender = vi
			.fn<PushSender>()
			.mockRejectedValueOnce(new Error('network'))
			.mockResolvedValueOnce(201);
		const res = await sendRelayPush(
			{ db, vapid: VAPID, sender, now: new Date('2026-07-17T12:00:00Z') },
			{ userId: 'u', tenantId: T, runId: 'r', runTitle: 'R', signal: APPROVAL }
		);
		expect(res.attempted).toBe(2);
		expect(res.delivered).toBe(1); // the second still went through
	});

	it('fail-open: a prefs read error returns a swallowed result, not a throw', async () => {
		const { db } = makeDb({
			notification_prefs: { data: null, error: { message: 'db down' } }
		});
		const sender = vi.fn<PushSender>();
		const res = await sendRelayPush(
			{ db, vapid: VAPID, sender, now: new Date('2026-07-17T12:00:00Z') },
			{ userId: 'u', tenantId: T, runId: 'r', runTitle: 'R', signal: APPROVAL }
		);
		expect(res.delivered).toBe(0);
		expect(sender).not.toHaveBeenCalled();
	});
});
