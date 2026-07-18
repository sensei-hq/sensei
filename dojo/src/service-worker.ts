// Dōjō Relay service worker (relay P4.3) — Web Push receive + click-through.
//
// Bundled + auto-registered by SvelteKit when this file exists at
// src/service-worker.ts (see docs/kit/service-workers). Built by
// adapter-cloudflare into the Worker output.
//
// Scope here is DELIBERATELY minimal — just the two push handlers:
//   push             → render the zero-knowledge notification
//   notificationclick → focus an existing tab or open the deep-link url
// No caching / offline logic lives here yet; that's P4.5. The actual push
// SEND (VAPID-signed, from the Worker, on a raised gate) is P4.4.
//
// Zero-knowledge invariant: the payload carries only a short logical message
// ({ title, body, url }) — "needs you / stalled / crashed on <run>" — and a
// deep-link. It NEVER contains code, diffs, or command text.

/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />
/// <reference types="@sveltejs/kit" />

// Give `self` the ServiceWorkerGlobalScope shape (see the SvelteKit SW docs).
const sw = globalThis.self as unknown as ServiceWorkerGlobalScope;

/** The push payload shape the P4.4 sender emits. Zero-knowledge by contract. */
interface RelayPushPayload {
	/** Short headline, e.g. "Approval needed". */
	title?: string;
	/** One-line logical message, e.g. "cargo test on Round-trip". Never code. */
	body?: string;
	/** Deep-link into the run/gate, e.g. "/console/relay/<run_id>". */
	url?: string;
	/** Opaque grouping tag so repeat gates on one run collapse (optional). */
	tag?: string;
}

/** Parse the push event's data as a RelayPushPayload, tolerating a bare string
 *  or malformed/empty data (a push with no body still shows a generic prompt). */
function parsePayload(event: PushEvent): RelayPushPayload {
	if (!event.data) return {};
	try {
		return event.data.json() as RelayPushPayload;
	} catch {
		// Non-JSON body → treat the text as the message.
		const text = event.data.text();
		return text ? { body: text } : {};
	}
}

sw.addEventListener('push', (event: PushEvent) => {
	const p = parsePayload(event);
	const title = p.title?.trim() || 'Dōjō Relay';
	const body = p.body?.trim() || 'A run needs you.';
	const url = typeof p.url === 'string' && p.url ? p.url : '/console/relay';
	event.waitUntil(
		sw.registration.showNotification(title, {
			body,
			icon: '/favicon.svg',
			badge: '/favicon.svg',
			// Collapse repeat notifications for the same run into one.
			tag: p.tag || url,
			// Carry the deep-link through to the click handler.
			data: { url }
		})
	);
});

sw.addEventListener('notificationclick', (event: NotificationEvent) => {
	event.notification.close();
	const data = (event.notification.data ?? {}) as { url?: string };
	const target = typeof data.url === 'string' && data.url ? data.url : '/console/relay';
	event.waitUntil(focusOrOpen(target));
});

/** Focus an already-open Dōjō tab (navigating it to the deep-link) or open a
 *  new window. Keeps the human in one tab rather than spawning a new one each tap. */
async function focusOrOpen(url: string): Promise<void> {
	const clientList = await sw.clients.matchAll({ type: 'window', includeUncontrolled: true });
	for (const client of clientList) {
		// Same-origin tab already open → focus it and steer it to the target.
		if ('focus' in client) {
			const windowClient = client as WindowClient;
			await windowClient.focus();
			if ('navigate' in windowClient && windowClient.navigate) {
				await windowClient.navigate(url).catch(() => {});
			}
			return;
		}
	}
	await sw.clients.openWindow(url);
}
