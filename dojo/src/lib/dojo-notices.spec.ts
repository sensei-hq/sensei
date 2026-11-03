import { describe, expect, it } from 'vitest';
import { inboxUnreachableNotice, actionFailedNotice, needsUnreadNotice, signInNotice, askSendNotice } from './dojo-notices';

// The dōjō's error copy for screens that take an error as a prop — one place,
// each a Notice (packages/ui) so every failure reads the same way.

describe('dōjō notices', () => {
	it('say nothing when nothing failed', () => {
		for (const f of [inboxUnreachableNotice, actionFailedNotice, needsUnreadNotice, askSendNotice]) expect(f(null)).toBeNull();
		expect(signInNotice(null, false)).toBeNull();
	});

	it('each failure is a fault with its own title and the reason as detail', () => {
		expect(inboxUnreachableNotice('timeout')).toEqual({ tone: 'fault', title: "Couldn't reach the dōjō", detail: 'timeout' });
		expect(actionFailedNotice('forbidden')).toEqual({ tone: 'fault', title: "That change didn't go through", detail: 'forbidden' });
		expect(needsUnreadNotice('service down')).toEqual({ tone: 'fault', title: "Couldn't load what needs a maintainer", detail: 'service down' });
		expect(askSendNotice('503')).toEqual({ tone: 'fault', title: "Couldn't send your answer", detail: '503' });
	});

	it('sign-in: a failure is a fault; the magic-link note is a normal notice', () => {
		expect(signInNotice('bad state', true)).toEqual({ tone: 'fault', title: "Couldn't sign in", detail: 'bad state' });
		expect(signInNotice('Check your email for the sign-in link.', false)).toEqual({ tone: 'normal', title: 'Check your email for the sign-in link.' });
	});
});
