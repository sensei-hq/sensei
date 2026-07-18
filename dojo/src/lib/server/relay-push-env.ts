// Relay P4.4 — env-coupled glue for the Web Push send. Kept SEPARATE from
// relay-push-send.ts so the pure-logic unit tests can import the send module
// without the SvelteKit `$env/dynamic/private` virtual module (which isn't
// generated under vitest). This file reads the VAPID keys from the Worker env and
// hands relay-push-send the fully-injected deps.
//
// VAPID keys (read from platform.env via $env/dynamic/{public,private}):
//   VAPID_PRIVATE_KEY — SECRET, base64url `d` scalar of the P-256 keypair. Local:
//                       gitignored .dev.vars. Prod: `wrangler secret put`.
//   VAPID_SUBJECT     — mailto: contact.
//   PUBLIC_VAPID_KEY  — base64url raw public key (also shipped to the browser).

import { env as pub } from '$env/dynamic/public';
import { env as priv } from '$env/dynamic/private';
import { dojoDb } from './dojo-supabase';
import {
	sendRelayPush,
	type PushDb,
	type SendArgs,
	type SendResult,
	type VapidKeys
} from './relay-push-send';

/** Read the VAPID keys from the Worker env, or null if not fully configured (so the
 *  caller can no-op cleanly rather than throw — a missing key must never break a
 *  gate raise). */
export function vapidFromEnv(): VapidKeys | null {
	const privateKey = priv.VAPID_PRIVATE_KEY;
	const subject = priv.VAPID_SUBJECT;
	const publicKey = pub.PUBLIC_VAPID_KEY;
	if (!privateKey || !subject || !publicKey) return null;
	return { privateKey, subject, publicKey };
}

/**
 * Fire a relay push for a signal, reading keys + the service-role db from the env.
 * Fail-open: returns a `gated` result (no throw) if VAPID isn't configured. Meant
 * to be handed to the CF adapter's `waitUntil` at the trigger site so it never
 * blocks or fails the daemon's raise.
 */
export async function sendRelayPushFromEnv(args: SendArgs): Promise<SendResult> {
	const vapid = vapidFromEnv();
	if (!vapid) {
		console.warn('relay-push: VAPID not configured — skipping push');
		return { gated: true, attempted: 0, delivered: 0, disabled: [] };
	}
	// dojoDb() is the service-role client; its query builder satisfies PushDb structurally.
	return sendRelayPush({ db: dojoDb() as unknown as PushDb, vapid }, args);
}
