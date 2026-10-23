// A value sealed for the browser to carry and only this server to read (#308).
//
//   * AES-GCM under a key HKDF-derived from the server's secret and a PURPOSE —
//     each use (the GitHub token cookie, the Azure connect state, the Azure
//     pending grant) has its own key, so a value sealed for one never opens as
//     another;
//   * BOUND to the Supabase login it was sealed for, and to an expiry, inside the
//     seal — another login's value, or a stale one, opens to nothing.
//
// The browser holds ciphertext it can neither read nor forge. Opening never
// throws: absent, tampered, expired, another login, another purpose or another
// secret all open to null.

const enc = new TextEncoder();
const dec = new TextDecoder();

export function b64url(bytes: Uint8Array): string {
	let s = '';
	for (const b of bytes) s += String.fromCharCode(b);
	return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function unb64url(s: string): Uint8Array | null {
	if (!/^[A-Za-z0-9_-]+$/.test(s)) return null;
	try {
		const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/'));
		return Uint8Array.from(bin, (c) => c.charCodeAt(0));
	} catch {
		return null;
	}
}

/** The AES-GCM key for one purpose, derived from the server secret — never the
 *  secret itself. */
async function sealKey(secret: string, purpose: string): Promise<CryptoKey> {
	if (!secret) throw new Error('sealed value: no server secret to seal with');
	const base = await crypto.subtle.importKey('raw', enc.encode(secret), 'HKDF', false, ['deriveKey']);
	return crypto.subtle.deriveKey(
		{ name: 'HKDF', hash: 'SHA-256', salt: enc.encode('dojo'), info: enc.encode(purpose) },
		base,
		{ name: 'AES-GCM', length: 256 },
		false,
		['encrypt', 'decrypt']
	);
}

/** Seal `value` for `authUserId`, under `purpose`, good for `ttlSeconds` from `now`. */
export async function seal(
	value: unknown,
	authUserId: string,
	secret: string,
	purpose: string,
	ttlSeconds: number,
	now = Date.now()
): Promise<string> {
	const key = await sealKey(secret, purpose);
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const payload = enc.encode(JSON.stringify({ t: value, u: authUserId, exp: now + ttlSeconds * 1000 }));
	const sealed = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, payload));
	const out = new Uint8Array(iv.length + sealed.length);
	out.set(iv);
	out.set(sealed, iv.length);
	return b64url(out);
}

/** The value sealed in `sealed`, or null when it is absent, tampered with,
 *  expired, sealed for another login or purpose, or under another secret. */
export async function openSealed<T = unknown>(
	sealed: string | undefined,
	authUserId: string,
	secret: string,
	purpose: string,
	now = Date.now()
): Promise<T | null> {
	const bytes = sealed ? unb64url(sealed) : null;
	if (!bytes || bytes.length < 13 || !secret) return null;
	try {
		const key = await sealKey(secret, purpose);
		const plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: bytes.slice(0, 12) }, key, bytes.slice(12));
		const p = JSON.parse(dec.decode(plain)) as { t?: unknown; u?: unknown; exp?: unknown };
		if (p.u !== authUserId || typeof p.exp !== 'number' || p.exp < now) return null;
		return (p.t ?? null) as T | null;
	} catch {
		return null;
	}
}

/** A random value of `bytes` bytes, base64url — a state, a PKCE verifier. */
export function randomToken(bytes = 32): string {
	return b64url(crypto.getRandomValues(new Uint8Array(bytes)));
}
