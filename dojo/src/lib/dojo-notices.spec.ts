import { describe, expect, it } from 'vitest';
import { inboxUnreachableNotice, actionFailedNotice, needsUnreadNotice, signInNotice, askSendNotice, runUnreadNotice, RUN_NOT_FOUND, dojoLoadNotice, routeErrorPrimary } from './dojo-notices';

// The dōjō's error copy for screens that take an error as a prop — one place,
// each a Notice (packages/ui) so every failure reads the same way.

describe('dōjō notices', () => {
	it('say nothing when nothing failed', () => {
		for (const f of [inboxUnreachableNotice, actionFailedNotice, needsUnreadNotice, askSendNotice]) expect(f(null)).toBeNull();
		expect(signInNotice(null, false)).toBeNull();
	});

	it('each failure is a fault with its own title and the reason as detail', () => {
		expect(inboxUnreachableNotice('timeout')).toEqual({ tone: 'fault', title: "Couldn't reach the dōjō", detail: 'timeout', remedy: 'Retry. If it keeps failing, reload the page.' });
		expect(actionFailedNotice('forbidden')).toEqual({ tone: 'fault', title: "That change didn't go through", detail: 'forbidden', remedy: 'Check what it says, then try again.' });
		expect(needsUnreadNotice('service down')).toEqual({ tone: 'fault', title: "Couldn't load what needs a maintainer", detail: 'service down', remedy: 'Retry. If it keeps failing, reload the page.' });
		expect(askSendNotice('503')).toEqual({ tone: 'fault', title: "Couldn't send your answer", detail: '503', remedy: 'Send it again. If it keeps failing, answer from the session instead.' });
	});

	it('sign-in: a failure is a fault; the magic-link note is a normal notice', () => {
		expect(signInNotice('bad state', true)).toEqual({ tone: 'fault', title: "Couldn't sign in", detail: 'bad state', remedy: 'Try again. If it keeps failing, use another way to sign in.' });
		expect(signInNotice('Check your email for the sign-in link.', false)).toEqual({ tone: 'normal', title: 'Check your email for the sign-in link.' });
	});

	it('a run that could not be read is a fault with Retry — never an empty state', () => {
		const retry = () => {};
		expect(runUnreadNotice('could not reach the dojo service', retry)).toEqual({
			tone: 'fault',
			title: 'Live relay is unavailable',
			detail: 'could not reach the dojo service',
			remedy: 'Retry. If it keeps failing, reload the page.',
			primary: { label: 'Retry', onclick: retry, name: 'retry' }
		});
		expect(runUnreadNotice(null, retry)).toBeNull();
	});

	it('a run that is not there is an honest empty that names the way back', () => {
		expect(RUN_NOT_FOUND).toEqual({
			kanji: '観',
			title: 'This run is not in your dōjōs.',
			description: 'It may have finished long ago, or belong to a dōjō you are not in.',
			action: { label: 'Back to the inbox', href: '/you', name: 'back' }
		});
	});

	// M3 (r1): a read that failed carries its status, so a refusal reads as one.
	it('a failed read with its status: 403 is a refusal that says who to ask; 0 says the dōjō did not answer', () => {
		expect(dojoLoadNotice('billing', { status: 403, message: 'admins only' })).toEqual({
			tone: 'refusal',
			title: "Couldn't load billing",
			detail: 'admins only',
			remedy: 'Ask an admin for access.',
			code: 'http_403'
		});
		expect(dojoLoadNotice('billing', { status: 0, message: 'could not reach the dojo service' })).toMatchObject({
			tone: 'fault',
			remedy: 'Check that the dōjō is running and reachable, then retry.'
		});
		expect(dojoLoadNotice('billing', 'boom')).toMatchObject({ tone: 'fault', detail: 'boom', remedy: 'Retry. If it keeps failing, reload the page.' });
		expect(dojoLoadNotice('billing', null)).toBeNull();
	});

	// M4 (r1): every notice the dōjō shows names its next step.
	it('every non-null notice has a remedy or a primary action', () => {
		const all = [
			inboxUnreachableNotice('x'), actionFailedNotice('x'), needsUnreadNotice('x'), askSendNotice('x'),
			signInNotice('x', true), runUnreadNotice('x', () => {}), dojoLoadNotice('x', 'y'),
			dojoLoadNotice('x', { status: 404, message: 'y' }), dojoLoadNotice('x', { status: 409, message: 'y' })
		];
		for (const n of all) expect(Boolean(n && ((n as { remedy?: string }).remedy || (n as { primary?: unknown }).primary)), JSON.stringify(n)).toBe(true);
	});

	// L4 (r1): a 401 page's next step is to sign in, not "Go to your dōjōs".
	it('a route error offers sign-in for a 401, else the way home', () => {
		expect(routeErrorPrimary(401)).toEqual({ label: 'Sign in', href: '/signin', name: 'signin' });
		expect(routeErrorPrimary(403)).toEqual({ label: 'Go to your dōjōs', href: '/', name: 'home' });
		expect(routeErrorPrimary(500)).toEqual({ label: 'Go to your dōjōs', href: '/', name: 'home' });
	});
});
