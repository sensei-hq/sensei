// Relay P4.4 — Web Push SEND (server-only). When the daemon raises a "needs-you"
// gate/stall (relay_inbox insert) or a run crashes (relay_sessions status), the
// Worker pushes a ZERO-KNOWLEDGE notification to the human's backgrounded devices.
//
// Split into PURE logic (prefs gating, endpoint dedup, payload build — unit-tested,
// no env/network) and a THIN injectable send (crypto + fetch + expired-disable).
// The VAPID (ES256 JWT) + payload encryption are delegated to
// @block65/webcrypto-web-push, a small Workers-safe (WebCrypto, no node built-ins)
// library — see relay-push-env.ts for how the keys are read from the Worker env.
// NOTE: that library encrypts with the `aesgcm` scheme (RFC 8291 draft-04), not the
// newer `aes128gcm` (RFC 8188); aesgcm is still accepted by the major push services.
// We pass the encoding through honestly (never relabel) — see normalizeHeaders.
// Callers run this in the background via the CF adapter's waitUntil so a send never
// blocks or fails the daemon's raise (fail-open).
//
// Payload contract (NEVER code/diffs/command text): { title, body, url } only —
// a logical "needs you / stalled / crashed on <run title>" + a deep link.

import { buildPushPayload } from '@block65/webcrypto-web-push';

/** A relay_inbox kind (mirrors dojo.relay_inbox_kind). */
export type RelayInboxKind = 'approval' | 'decision' | 'chat' | 'nudge' | 'stall';

/** The push event opt-in keys (mirrors notification_prefs.events + the P4.3 prefs route). */
export type PushEvent = 'approvals' | 'decisions' | 'stalls' | 'crashed';

/** A "needs-you" signal: an agent-raised inbox kind, or a run crash (status path). */
export type PushSignal = { type: 'inbox'; kind: RelayInboxKind } | { type: 'crashed' };

/** notification_prefs row shape (the subset the send reads). */
export type NotificationPrefs = {
	events?: Record<string, unknown> | null;
	quiet_hours?: { tz?: string; start?: string; end?: string } | null;
	muted_tenants?: string[] | null;
};

/** A stored push_subscriptions row (web_push jsonb). */
export type StoredSubscription = {
	id: string;
	web_push: { endpoint: string; p256dh: string; auth: string } | null;
};

/** The zero-knowledge push payload. Deliberately has NO code/diff/command fields.
 *  `tag` collapses repeat notifications for one run (the SW uses it — P4.3). */
export type RelayPushPayload = { title: string; body: string; url: string; tag: string };

// ---------------------------------------------------------------------------
// PURE logic (unit-tested — no env, no network, no clock beyond the passed `now`)
// ---------------------------------------------------------------------------

/** Map a "needs-you" signal to the notification_prefs event opt-in key it honours.
 *  `chat` / `nudge` are NOT needs-you (free-form / human-initiated) → null (no push). */
export function eventForSignal(signal: PushSignal): PushEvent | null {
	if (signal.type === 'crashed') return 'crashed';
	switch (signal.kind) {
		case 'approval': return 'approvals';
		case 'decision': return 'decisions';
		case 'stall': return 'stalls';
		default: return null; // chat, nudge — never push
	}
}

/** Is `now` inside a [start,end) HH:MM quiet-hours window in the given tz? Handles
 *  the wrap-past-midnight case (start > end, e.g. 22:00→07:00). Returns false if the
 *  window is absent/malformed (fail-open: absent quiet-hours never suppresses). */
export function inQuietHours(
	quiet: NotificationPrefs['quiet_hours'],
	now: Date
): boolean {
	if (!quiet || typeof quiet.start !== 'string' || typeof quiet.end !== 'string') return false;
	const start = parseHhMm(quiet.start);
	const end = parseHhMm(quiet.end);
	if (start === null || end === null) return false;
	if (start === end) return false; // zero-width window → never suppress
	const cur = minutesInTz(now, quiet.tz);
	if (cur === null) return false; // bad tz → don't suppress
	// Non-wrapping window (e.g. 09:00→17:00): inside iff start <= cur < end.
	if (start < end) return cur >= start && cur < end;
	// Wrapping window (e.g. 22:00→07:00): inside iff cur >= start OR cur < end.
	return cur >= start || cur < end;
}

/** "HH:MM" → minutes-since-midnight, or null if malformed. */
function parseHhMm(s: string): number | null {
	const m = /^(\d{1,2}):(\d{2})$/.exec(s.trim());
	if (!m) return null;
	const h = Number(m[1]);
	const min = Number(m[2]);
	if (h < 0 || h > 23 || min < 0 || min > 59) return null;
	return h * 60 + min;
}

/** Minutes-since-midnight of `now` in IANA `tz` (defaults to UTC), or null on a bad tz. */
function minutesInTz(now: Date, tz?: string): number | null {
	try {
		const fmt = new Intl.DateTimeFormat('en-US', {
			timeZone: tz || 'UTC',
			hour: '2-digit',
			minute: '2-digit',
			hour12: false
		});
		const parts = fmt.formatToParts(now);
		const h = Number(parts.find((p) => p.type === 'hour')?.value);
		const min = Number(parts.find((p) => p.type === 'minute')?.value);
		if (!Number.isFinite(h) || !Number.isFinite(min)) return null;
		// Intl can emit "24" for the midnight hour with hour12:false — normalise.
		return ((h % 24) * 60 + min) % (24 * 60);
	} catch {
		return null; // invalid time zone
	}
}

/** Should a push fire for this signal, given the user's prefs + the run's tenant?
 *  Gates on: the event opt-in (default OFF — must be explicitly true), the tenant
 *  NOT being muted, and NOT being inside quiet-hours. `chat`/`nudge` never push. */
export function shouldPush(
	signal: PushSignal,
	prefs: NotificationPrefs | null | undefined,
	tenantId: string,
	now: Date
): boolean {
	const event = eventForSignal(signal);
	if (!event) return false; // not a needs-you kind
	if (!prefs) return false; // no prefs row → not opted in
	// Event opt-in: must be explicitly true (default OFF).
	if (prefs.events?.[event] !== true) return false;
	// Muted tenant → silence.
	if (Array.isArray(prefs.muted_tenants) && prefs.muted_tenants.includes(tenantId)) return false;
	// Quiet hours → silence.
	if (inQuietHours(prefs.quiet_hours, now)) return false;
	return true;
}

/** Dedup subscriptions by web_push.endpoint (two rows for one endpoint push once);
 *  drop rows without a usable {endpoint,p256dh,auth}. Keeps the first occurrence. */
export function dedupByEndpoint(subs: StoredSubscription[]): StoredSubscription[] {
	const seen = new Set<string>();
	const out: StoredSubscription[] = [];
	for (const s of subs) {
		const wp = s.web_push;
		if (!wp || !wp.endpoint || !wp.p256dh || !wp.auth) continue;
		if (seen.has(wp.endpoint)) continue;
		seen.add(wp.endpoint);
		out.push(s);
	}
	return out;
}

/** Build the ZERO-KNOWLEDGE push payload for a signal + run. `url` deep-links to the
 *  run's relay console. NEVER embeds code/diffs/command text — only a logical label
 *  and the run TITLE (already stripped by the daemon before it crosses the wire). */
export function buildRelayPushPayload(
	signal: PushSignal,
	runTitle: string | null | undefined,
	runId: string
): RelayPushPayload {
	const label = signalLabel(signal);
	const run = runTitle && runTitle.trim() ? runTitle.trim() : 'your run';
	const url = `/console/relay/${runId}`;
	return {
		// Title matches the service worker's default (P4.3) for a consistent surface.
		title: 'Dōjō Relay',
		body: `${label} on ${run}`,
		url,
		// Collapse repeat notifications for the same run into one (SW reads `tag`).
		tag: url
	};
}

/** The human-readable verb for a signal (drives the notification body). */
function signalLabel(signal: PushSignal): string {
	if (signal.type === 'crashed') return 'crashed';
	switch (signal.kind) {
		case 'approval': return 'needs you';
		case 'decision': return 'needs a decision';
		case 'stall': return 'stalled';
		default: return 'needs you';
	}
}

// ---------------------------------------------------------------------------
// IMPURE send (thin + injectable — the pure tests never touch real keys/network)
// ---------------------------------------------------------------------------

/** VAPID material for the send (base64url public/private + a mailto subject). */
export type VapidKeys = { subject: string; publicKey: string; privateKey: string };

/** Minimal supabase-js surface the send needs (service-role dojoDb). Structural so
 *  a test can pass a stub without importing the real client. */
export type PushDb = {
	from(table: string): PushDbQuery;
};
export type PushDbQuery = {
	select(cols: string): PushDbQuery;
	update(patch: Record<string, unknown>): PushDbQuery;
	eq(col: string, val: unknown): PushDbQuery;
};

/** A single push HTTP send — injectable so tests can assert the request shape
 *  against a mock/echo endpoint without real keys/network. Returns the HTTP status. */
export type PushSender = (
	sub: StoredSubscription,
	payload: RelayPushPayload,
	vapid: VapidKeys
) => Promise<number>;

/** The default sender: encrypts + VAPID-signs via the Workers-safe library, then
 *  POSTs to the push service. The VAPID Authorization is normalised to the RFC 8292
 *  form (`vapid t=<ES256 JWT>, k=<vapid-public>`); the encrypted body + its content
 *  encoding come straight from the library (see {@link normalizeHeaders}). */
export const defaultPushSender: PushSender = async (sub, payload, vapid) => {
	const wp = sub.web_push!;
	const built = await buildPushPayload(
		{ data: payload, options: { ttl: 12 * 60 * 60, urgency: 'high' } },
		{ endpoint: wp.endpoint, expirationTime: null, keys: { auth: wp.auth, p256dh: wp.p256dh } },
		{ subject: vapid.subject, publicKey: vapid.publicKey, privateKey: vapid.privateKey }
	);
	const headers = normalizeHeaders(built.headers, vapid.publicKey);
	const res = await fetch(wp.endpoint, {
		method: built.method,
		headers,
		body: built.body as unknown as BodyInit
	});
	return res.status;
};

/** Shape the push request headers from the library's output. Two jobs:
 *
 *  1. VAPID auth → RFC 8292 form: the library emits the draft-01
 *     `Authorization: WebPush <jwt>` + `Crypto-Key: p256ecdsa=<key>`; we rewrite it
 *     to the final single-header `Authorization: vapid t=<jwt>, k=<vapid-public>`.
 *
 *  2. Pass the CONTENT-ENCODING and its companion headers THROUGH verbatim so the
 *     header always matches the body. @block65/webcrypto-web-push encrypts with the
 *     `aesgcm` scheme (RFC 8291 draft-04): `Content-Encoding: aesgcm` with the salt
 *     in `Encryption` and the server DH key in `Crypto-Key: dh=…;p256ecdsa=…`. That
 *     is NOT the newer RFC 8188 `aes128gcm` (which frames salt+key inside the body).
 *     `aesgcm` remains accepted by the major push services (FCM/Mozilla/WebKit). We
 *     never relabel it `aes128gcm` — a mismatched header would break decryption.
 *     (Deviation from the plan's stated `aes128gcm` — flagged for Jerry; migrating
 *     to an aes128gcm sender is a follow-up if a target push service rejects aesgcm.)
 *
 *  Exported for unit-testing the header shape without a live send. */
export function normalizeHeaders(
	built: Record<string, string>,
	vapidPublicKey: string
): Record<string, string> {
	const rawAuth = built['authorization'] ?? built['Authorization'] ?? '';
	const jwt = rawAuth.replace(/^WebPush\s+/i, '').trim();
	const out: Record<string, string> = {
		Authorization: `vapid t=${jwt}, k=${vapidPublicKey}`,
		// Pass the library's encryption headers through unchanged (header ↔ body match).
		'Content-Encoding': built['content-encoding'] ?? 'aesgcm',
		'Content-Type': built['content-type'] ?? 'application/octet-stream',
		TTL: built['ttl'] ?? String(12 * 60 * 60)
	};
	// aesgcm companions — REQUIRED for the push service to decrypt an aesgcm body.
	if (built['encryption']) out['Encryption'] = built['encryption'];
	if (built['crypto-key']) out['Crypto-Key'] = built['crypto-key'];
	if (built['urgency']) out['Urgency'] = built['urgency'];
	if (built['content-length']) out['Content-Length'] = built['content-length'];
	return out;
}

/** Dependencies for {@link sendRelayPush} — all injectable for tests. */
export type SendDeps = {
	db: PushDb;
	vapid: VapidKeys;
	/** Defaults to {@link defaultPushSender}; tests inject a mock. */
	sender?: PushSender;
	/** Defaults to `new Date()`; tests inject a fixed clock. */
	now?: Date;
};

export type SendArgs = {
	userId: string;
	tenantId: string;
	runId: string;
	runTitle: string | null | undefined;
	signal: PushSignal;
};

export type SendResult = {
	/** true if the signal was gated out (prefs / muted / quiet-hours / not needs-you). */
	gated: boolean;
	/** number of subscriptions a push was attempted for (after dedup). */
	attempted: number;
	/** number of 2xx sends. */
	delivered: number;
	/** subscription ids disabled due to 404/410 (expired endpoints). */
	disabled: string[];
};

/**
 * Load the user's ENABLED subscriptions, apply prefs gating + endpoint dedup, and
 * send a zero-knowledge push to each. Fail-open + swallow: any thrown error is
 * caught and logged so a push failure NEVER breaks the caller (the daemon's gate
 * raise). 404/410 from a push service = expired endpoint → the row's `enabled` is
 * flipped false (service-role update). Intended to run in the background via the
 * CF adapter's waitUntil.
 */
export async function sendRelayPush(deps: SendDeps, args: SendArgs): Promise<SendResult> {
	const now = deps.now ?? new Date();
	const sender = deps.sender ?? defaultPushSender;
	const result: SendResult = { gated: false, attempted: 0, delivered: 0, disabled: [] };
	try {
		// Prefs first — a cheap read that can short-circuit the whole send.
		const { data: prefsRow, error: prefsErr } = await asPromise(
			deps.db
				.from('notification_prefs')
				.select('events, quiet_hours, muted_tenants')
				.eq('user_id', args.userId)
		);
		if (prefsErr) throw new Error(`notification_prefs read failed: ${errMsg(prefsErr)}`);
		const prefs = (firstRow(prefsRow) as NotificationPrefs | null) ?? null;

		if (!shouldPush(args.signal, prefs, args.tenantId, now)) {
			result.gated = true;
			return result;
		}

		// The user's enabled subscriptions.
		const { data: subRows, error: subErr } = await asPromise(
			deps.db
				.from('push_subscriptions')
				.select('id, web_push')
				.eq('user_id', args.userId)
				.eq('enabled', true)
		);
		if (subErr) throw new Error(`push_subscriptions read failed: ${errMsg(subErr)}`);
		const subs = dedupByEndpoint((subRows as StoredSubscription[]) ?? []);

		const payload = buildRelayPushPayload(args.signal, args.runTitle, args.runId);

		for (const sub of subs) {
			result.attempted++;
			try {
				const status = await sender(sub, payload, deps.vapid);
				if (status >= 200 && status < 300) {
					result.delivered++;
				} else if (status === 404 || status === 410) {
					// Expired / gone subscription — disable it so we stop trying.
					await disableSubscription(deps.db, sub.id);
					result.disabled.push(sub.id);
				} else {
					console.warn('relay-push: send returned non-2xx', { id: sub.id, status });
				}
			} catch (e) {
				// One bad subscription must not sink the rest.
				console.warn('relay-push: send threw', { id: sub.id }, e);
			}
		}
		return result;
	} catch (e) {
		// Fail-open: a push failure never breaks the caller (the gate raise).
		console.warn('relay-push: send aborted', e);
		return result;
	}
}

async function disableSubscription(db: PushDb, id: string): Promise<void> {
	const { error } = await asPromise(
		db
			.from('push_subscriptions')
			.update({ enabled: false, updated_at: new Date().toISOString() })
			.eq('id', id)
	);
	if (error) console.warn('relay-push: could not disable subscription', { id, error: errMsg(error) });
}

// --- small helpers over the supabase-js thenable query builder --------------

/** supabase-js query builders are thenable; await one to get `{ data, error }`. */
function asPromise<T = unknown>(
	q: unknown
): Promise<{ data: T | null; error: unknown }> {
	return Promise.resolve(q as Promise<{ data: T | null; error: unknown }>);
}

/** A `.select()` may resolve to an array (0..n rows); take the first (prefs is 0..1). */
function firstRow(data: unknown): unknown {
	if (Array.isArray(data)) return data[0] ?? null;
	return data ?? null;
}

function errMsg(e: unknown): string {
	if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
	return String(e);
}
