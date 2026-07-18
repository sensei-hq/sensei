// Web Push subscribe flow (relay P4.3) — the CLIENT half. Registers the service
// worker, subscribes the browser's PushManager with the VAPID public key, and
// POSTs the resulting subscription to the Worker so it lands in
// dojo.push_subscriptions. The SEND (P4.4) is out of scope here.
//
// Structure: the PURE, browser-free bits (base64url→Uint8Array, POST-body
// shaping) are exported standalone so they unit-test without a DOM; the impure
// orchestrator (subscribeToPush) guards for unsupported browsers and returns a
// typed result rather than throwing on the "can't" paths.
//
// Reuses the shared HTTP primitives (dojoApiUrl / encodeTenant / authHeaders /
// parseJson) from dojo-api.ts — same pattern as relay-data.ts, no duplication.

import { authHeaders, dojoApiUrl, encodeTenant, parseJson, type DojoCallOpts } from './dojo-api';

/** The POST body the /relay/push/subscribe Worker route accepts. Mirrors the
 *  web_push jsonb column ({endpoint, p256dh, auth}) plus the platform tag. */
export interface PushSubscribeBody {
	endpoint: string;
	keys: { p256dh: string; auth: string };
	platform: 'web';
}

/** Outcome of {@link subscribeToPush}. `ok` carries the stored subscription id;
 *  the non-ok reasons let the UI show the right message without a try/catch. */
export type SubscribeResult =
	| { ok: true; id: string }
	| { ok: false; reason: 'unsupported' | 'denied' | 'no-vapid-key' | 'error'; message: string };

// ── Pure helpers (browser-free, unit-tested) ─────────────────────────────────

/**
 * Convert a base64url-encoded VAPID application-server key to the Uint8Array
 * that `PushManager.subscribe({ applicationServerKey })` requires. This is the
 * canonical web-push helper: pad to a multiple of 4, swap the URL-safe alphabet
 * back to standard base64, then decode each byte. Throws on non-base64 input.
 */
export function urlBase64ToUint8Array(base64String: string): Uint8Array<ArrayBuffer> {
	const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
	const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
	const rawData = atob(base64);
	// Back it by an explicit ArrayBuffer so the result is a `BufferSource` that
	// PushManager.subscribe's `applicationServerKey` accepts (a SharedArrayBuffer-
	// backed Uint8Array would not satisfy the DOM type).
	const output = new Uint8Array(new ArrayBuffer(rawData.length));
	for (let i = 0; i < rawData.length; i++) output[i] = rawData.charCodeAt(i);
	return output;
}

/**
 * Shape a browser `PushSubscription` into the Worker's store payload. Splits the
 * subscription's `getKey('p256dh')`/`getKey('auth')` ArrayBuffers out to
 * base64url strings (what the DDL's web_push jsonb holds). Pure over its inputs:
 * takes the already-extracted endpoint + key buffers so it needs no DOM.
 */
export function subscriptionToStorePayload(
	endpoint: string,
	p256dh: ArrayBuffer | null,
	auth: ArrayBuffer | null
): PushSubscribeBody {
	if (!endpoint) throw new Error('push subscription is missing its endpoint');
	if (!p256dh || !auth) throw new Error('push subscription is missing its p256dh/auth keys');
	return {
		endpoint,
		keys: { p256dh: arrayBufferToBase64Url(p256dh), auth: arrayBufferToBase64Url(auth) },
		platform: 'web'
	};
}

/** base64url-encode an ArrayBuffer (no padding, URL-safe alphabet). */
export function arrayBufferToBase64Url(buffer: ArrayBuffer): string {
	const bytes = new Uint8Array(buffer);
	let binary = '';
	for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** True when this browser can do Web Push at all (SW + PushManager + Notification). */
export function pushSupported(): boolean {
	return (
		typeof navigator !== 'undefined' &&
		'serviceWorker' in navigator &&
		typeof window !== 'undefined' &&
		'PushManager' in window &&
		'Notification' in window
	);
}

// ── Impure orchestrator ──────────────────────────────────────────────────────

/** Convert a live PushSubscription to the store payload (thin DOM adapter over
 *  the pure {@link subscriptionToStorePayload}). Exported for the UI's re-sync path. */
export function pushSubscriptionToBody(sub: PushSubscription): PushSubscribeBody {
	return subscriptionToStorePayload(sub.endpoint, sub.getKey('p256dh'), sub.getKey('auth'));
}

/** POST a subscription to the Worker route; returns the stored row id. */
export async function postSubscription(
	tenantKey: string,
	body: PushSubscribeBody,
	opts: DojoCallOpts = {}
): Promise<{ id: string }> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/relay/push/subscribe`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
		body: JSON.stringify(body)
	});
	return parseJson<{ id: string }>(res);
}

/** Per-event push opt-ins the notification_prefs route accepts. */
export interface NotificationPrefEvents {
	approvals?: boolean;
	decisions?: boolean;
	stalls?: boolean;
	crashed?: boolean;
}

/** POST the caller's push event opt-ins; the route merges onto existing prefs. */
export async function postNotificationPrefs(
	tenantKey: string,
	events: NotificationPrefEvents,
	opts: DojoCallOpts = {}
): Promise<void> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/relay/push/prefs`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
		body: JSON.stringify({ events })
	});
	await parseJson<{ user_id: string }>(res);
}

/**
 * The full subscribe flow: ensure the SW is ready, subscribe the PushManager
 * with the VAPID public key, and store the subscription via the Worker route.
 * Never throws on the "can't" paths — returns a typed {@link SubscribeResult} so
 * the UI can message unsupported / permission-denied / missing-key distinctly.
 *
 * @param tenantKey  the `<origin>/<org>` discovery path (same as relay-data).
 * @param vapidPublicKey  the PUBLIC_VAPID_KEY (base64url application-server key).
 * @param opts  accessToken (bearer) + injectable fetch.
 */
export async function subscribeToPush(
	tenantKey: string,
	vapidPublicKey: string | undefined,
	opts: DojoCallOpts = {}
): Promise<SubscribeResult> {
	if (!pushSupported()) {
		return { ok: false, reason: 'unsupported', message: 'This browser does not support push notifications.' };
	}
	if (!vapidPublicKey) {
		return { ok: false, reason: 'no-vapid-key', message: 'Push is not configured (missing VAPID key).' };
	}

	try {
		// SvelteKit auto-registers src/service-worker.ts; `ready` resolves once it's
		// active. matchAll/register fallback keeps this robust if registration lags.
		const registration =
			(await navigator.serviceWorker.getRegistration()) ??
			(await navigator.serviceWorker.register('/service-worker.js', { type: 'module' }));
		await navigator.serviceWorker.ready;

		const permission = await Notification.requestPermission();
		if (permission !== 'granted') {
			return { ok: false, reason: 'denied', message: 'Notifications are blocked for this site.' };
		}

		// Reuse an existing subscription if present (idempotent re-enable), else
		// subscribe fresh with the VAPID key.
		const existing = await registration.pushManager.getSubscription();
		const subscription =
			existing ??
			(await registration.pushManager.subscribe({
				userVisibleOnly: true,
				applicationServerKey: urlBase64ToUint8Array(vapidPublicKey)
			}));

		const { id } = await postSubscription(tenantKey, pushSubscriptionToBody(subscription), opts);
		return { ok: true, id };
	} catch (e) {
		const message = e instanceof Error ? e.message : 'Could not enable notifications.';
		return { ok: false, reason: 'error', message };
	}
}
