import { describe, it, expect, vi } from 'vitest';
import { RelayNotify } from './relay-notify.svelte';

// "Notify me" in the relay inbox header (user, 2026-10-28). The inbox spans every
// dōjō you belong to, and a push subscription is stored per dōjō — so enabling
// registers this browser with each. The state machine lived in the component
// (audit 2026-10-28, item 13); it is RelayNotify's.

function harness(over: Record<string, unknown> = {}) {
	const deps = {
		supported: () => true,
		permission: () => 'default' as NotificationPermission,
		subscribe: vi.fn(async () => ({ ok: true as const, id: 's1' })),
		postPrefs: vi.fn(async () => {}),
		...over
	};
	return { notify: new RelayNotify(deps as never), deps };
}

describe('RelayNotify', () => {
	it('starts from what the browser can do and has allowed', () => {
		expect(harness().notify.view).toMatchObject({ kind: 'button', buttonLabel: 'Notify me' });
		expect(harness({ supported: () => false }).notify.view).toMatchObject({ kind: 'status', text: 'Notifications not supported here' });
		expect(harness({ permission: () => 'denied' }).notify.view).toMatchObject({
			kind: 'status',
			text: 'Notifications blocked — enable them in your browser settings'
		});
		expect(harness({ permission: () => 'granted' }).notify.view).toMatchObject({ kind: 'on', text: 'Notifications on' });
	});

	it('registers this browser with every dōjō in the inbox', async () => {
		const h = harness();
		await h.notify.enable(['github.com/acme', 'github.com/other']);
		expect(h.deps.subscribe).toHaveBeenCalledTimes(2);
		expect(h.deps.postPrefs).toHaveBeenCalledTimes(2);
		expect(h.notify.view).toMatchObject({ kind: 'on', text: 'Notifications on' });
	});

	it('says which dōjōs it could not register with', async () => {
		const h = harness({
			subscribe: vi.fn(async (t: string) =>
				t === 'github.com/other' ? { ok: false, reason: 'error', message: '503' } : { ok: true, id: 's1' }
			)
		});
		await h.notify.enable(['github.com/acme', 'github.com/other']);
		expect(h.notify.view).toMatchObject({ kind: 'on', text: 'Notifications on for 1 of 2 dōjōs — github.com/other: 503' });
	});

	it('a browser that refuses is blocked, not an error', async () => {
		const h = harness({ subscribe: vi.fn(async () => ({ ok: false, reason: 'denied', message: 'blocked' })) });
		await h.notify.enable(['github.com/acme']);
		expect(h.notify.view).toMatchObject({ kind: 'status', text: expect.stringMatching(/blocked/) });
	});

	it('preferences that did not save do not undo a good subscription', async () => {
		const h = harness({ postPrefs: vi.fn(async () => { throw new Error('x'); }) });
		await h.notify.enable(['github.com/acme']);
		expect(h.notify.view).toMatchObject({ kind: 'on', text: 'Notifications on (preferences not saved)' });
	});

	it('the button says it is working, and nothing to enable is said', async () => {
		let release: () => void = () => {};
		const gate = new Promise<void>((r) => (release = r));
		const h = harness({ subscribe: vi.fn(async () => { await gate; return { ok: true, id: 's' }; }) });
		const run = h.notify.enable(['github.com/acme']);
		expect(h.notify.view).toMatchObject({ kind: 'button', buttonLabel: 'Enabling…', busy: true });
		release();
		await run;
		const none = harness();
		await none.notify.enable([]);
		expect(none.notify.view).toMatchObject({ kind: 'button', error: 'No dōjō to notify you about yet.' });
	});
});
