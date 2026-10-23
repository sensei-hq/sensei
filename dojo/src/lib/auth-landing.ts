// #329. After a magic link, the browser lands on a URL whose fragment carries the
// new session (`#access_token=…`). A fragment never reaches the server, so the
// first render uses the cookie the browser still holds: the previous account's.
// kavach then posts the new session and reloads the data (`invalidateAll`). Until
// that reload has finished, the page stays hidden: app.html marks <html> pending
// before first paint, and the root layout reveals it through `revealOnSync`.

/** `data-auth-pending` on <html> while a landing's session is not yet the server's. */
export const AUTH_PENDING_ATTR = 'data-auth-pending';

/** A fragment carrying a new session. app.html repeats this source verbatim. */
export const AUTH_LANDING_FRAGMENT = /(^#|&)access_token=/;

/** Longest the page stays hidden when no sync completes; the server then genuinely
 *  still holds the old session, so revealing it shows nothing stale. */
export const REVEAL_AFTER_MS = 10_000;

export function isAuthLanding(hash: string): boolean {
	return AUTH_LANDING_FRAGMENT.test(hash);
}

interface PendingRoot {
	hasAttribute(name: string): boolean;
	removeAttribute(name: string): void;
}

type Schedule = (fn: () => void, ms: number) => () => void;

const timer: Schedule = (fn, ms) => {
	const t = setTimeout(fn, ms);
	return () => clearTimeout(t);
};

/**
 * Wrap kavach's `invalidateAll` so the first completed reload reveals the page.
 * Also reveals after `REVEAL_AFTER_MS` if no sync completes. Later reloads pass
 * straight through.
 */
export function revealOnSync(
	root: PendingRoot,
	invalidate: () => Promise<void>,
	schedule: Schedule = timer
): () => Promise<void> {
	let cancel: (() => void) | null = null;
	const reveal = () => {
		cancel?.();
		cancel = null;
		root.removeAttribute(AUTH_PENDING_ATTR);
	};
	if (root.hasAttribute(AUTH_PENDING_ATTR)) cancel = schedule(reveal, REVEAL_AFTER_MS);
	return async () => {
		await invalidate();
		if (root.hasAttribute(AUTH_PENDING_ATTR)) reveal();
	};
}
