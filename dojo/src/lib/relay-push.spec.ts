import { describe, it, expect, vi } from 'vitest';
import {
	urlBase64ToUint8Array,
	arrayBufferToBase64Url,
	subscriptionToStorePayload,
	postSubscription,
	postNotificationPrefs,
	pushSupported,
	subscribeToPush
} from './relay-push';

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

/** Encode a byte array to base64url (test-side oracle, independent of the impl). */
function bytesToB64Url(bytes: number[]): string {
	let binary = '';
	for (const b of bytes) binary += String.fromCharCode(b);
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

describe('relay-push pure helpers', () => {
	it('urlBase64ToUint8Array round-trips a base64url VAPID key back to its bytes', () => {
		const bytes = [4, 160, 134, 245, 100, 6, 122, 255, 0, 128, 63];
		const b64url = bytesToB64Url(bytes);
		const out = urlBase64ToUint8Array(b64url);
		expect(Array.from(out)).toEqual(bytes);
	});

	it('urlBase64ToUint8Array handles the URL-safe alphabet and missing padding', () => {
		// Bytes chosen so the standard base64 contains both `+` (→ `-`) and `/` (→ `_`).
		const bytes = [251, 255, 190]; // "+/++"-ish → exercises URL-safe swap
		const out = urlBase64ToUint8Array(bytesToB64Url(bytes));
		expect(Array.from(out)).toEqual(bytes);
	});

	it('urlBase64ToUint8Array decodes the real dev VAPID public key to a 65-byte P-256 point', () => {
		const devKey =
			'BKCG9WQGeqIegaI3z54SBQWlwFi6C_x0LLhOAfs0p0ApYsNgvEGeiEou9QHjbWH681Tmv13ZmdveKXzWx9RxPQs';
		const out = urlBase64ToUint8Array(devKey);
		expect(out.length).toBe(65); // uncompressed EC point
		expect(out[0]).toBe(0x04); // uncompressed-point prefix
	});

	it('arrayBufferToBase64Url encodes bytes URL-safe with no padding', () => {
		const buf = new Uint8Array([251, 255, 190, 1]).buffer;
		const s = arrayBufferToBase64Url(buf);
		expect(s).not.toMatch(/[+/=]/);
		// round-trips back through the decoder
		expect(Array.from(urlBase64ToUint8Array(s))).toEqual([251, 255, 190, 1]);
	});

	it('subscriptionToStorePayload shapes endpoint + keys into the web_push body', () => {
		const p256dh = new Uint8Array([1, 2, 3, 4]).buffer;
		const auth = new Uint8Array([9, 8, 7]).buffer;
		const body = subscriptionToStorePayload('https://push.example/abc', p256dh, auth);
		expect(body.platform).toBe('web');
		expect(body.endpoint).toBe('https://push.example/abc');
		expect(body.keys.p256dh).toBe(bytesToB64Url([1, 2, 3, 4]));
		expect(body.keys.auth).toBe(bytesToB64Url([9, 8, 7]));
	});

	it('subscriptionToStorePayload rejects a missing endpoint', () => {
		expect(() => subscriptionToStorePayload('', new ArrayBuffer(4), new ArrayBuffer(4))).toThrow(/endpoint/);
	});

	it('subscriptionToStorePayload rejects missing keys', () => {
		expect(() => subscriptionToStorePayload('https://push.example/abc', null, null)).toThrow(/p256dh\/auth/);
	});
});

describe('relay-push postSubscription', () => {
	it('POSTs the body to relay/push/subscribe with the bearer token and returns the id', async () => {
		const calls: { url: string; init?: RequestInit }[] = [];
		const fetch = vi.fn(async (url: string, init?: RequestInit) => {
			calls.push({ url, init });
			return jsonResponse({ id: 'sub-1' });
		}) as unknown as typeof globalThis.fetch;
		const out = await postSubscription(
			'personal/jerry',
			{ endpoint: 'https://push.example/abc', keys: { p256dh: 'PK', auth: 'AK' }, platform: 'web' },
			{ accessToken: 'JWT', fetch }
		);
		expect(out).toEqual({ id: 'sub-1' });
		expect(calls[0].url).toContain('/v1/t/personal/jerry/relay/push/subscribe');
		expect(calls[0].init?.method).toBe('POST');
		expect((calls[0].init?.headers as Record<string, string>).Authorization).toBe('Bearer JWT');
		const sent = JSON.parse(String(calls[0].init?.body));
		expect(sent.endpoint).toBe('https://push.example/abc');
		expect(sent.platform).toBe('web');
	});

	it('throws on a non-2xx response', async () => {
		const fetch = vi.fn(async () => jsonResponse({ error: 'endpoint is required' }, 400)) as unknown as typeof globalThis.fetch;
		await expect(
			postSubscription('personal/jerry', { endpoint: 'x', keys: { p256dh: 'a', auth: 'b' }, platform: 'web' }, { fetch })
		).rejects.toThrow(/endpoint is required/);
	});
});

describe('relay-push postNotificationPrefs', () => {
	it('POSTs the events opt-ins to relay/push/prefs with the bearer token', async () => {
		const calls: { url: string; init?: RequestInit }[] = [];
		const fetch = vi.fn(async (url: string, init?: RequestInit) => {
			calls.push({ url, init });
			return jsonResponse({ user_id: 'u1', events: { approvals: true } });
		}) as unknown as typeof globalThis.fetch;
		await postNotificationPrefs('personal/jerry', { approvals: true, stalls: true, crashed: true }, { accessToken: 'JWT', fetch });
		expect(calls[0].url).toContain('/v1/t/personal/jerry/relay/push/prefs');
		expect(calls[0].init?.method).toBe('POST');
		expect((calls[0].init?.headers as Record<string, string>).Authorization).toBe('Bearer JWT');
		const sent = JSON.parse(String(calls[0].init?.body));
		expect(sent.events).toEqual({ approvals: true, stalls: true, crashed: true });
	});
});

describe('relay-push subscribeToPush guards (jsdom: no PushManager)', () => {
	it('pushSupported is false when the environment lacks PushManager', () => {
		// jsdom has navigator + window but no serviceWorker/PushManager → unsupported.
		expect(pushSupported()).toBe(false);
	});

	it('subscribeToPush returns an unsupported result instead of throwing', async () => {
		const fetch = vi.fn() as unknown as typeof globalThis.fetch;
		const res = await subscribeToPush('personal/jerry', 'VAPID', { fetch });
		expect(res.ok).toBe(false);
		if (!res.ok) expect(res.reason).toBe('unsupported');
		expect(fetch).not.toHaveBeenCalled();
	});
});
