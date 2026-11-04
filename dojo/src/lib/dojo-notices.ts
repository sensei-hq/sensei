// The dōjō's error copy for screens that take an error as a prop (no state
// class of their own): each failure becomes a Notice (packages/ui), so every
// one reads the same way — a title saying what failed, the reason as detail.
// Pure, so the copy unit-tests without a DOM.
import { failureNotice, loadFailedNotice } from '@sensei/ui';
import type { ReadError } from './read-error';

/** The inbox could not be read from the dōjō. */
export const inboxUnreachableNotice = (error: string | null) => failureNotice("Couldn't reach the dōjō", error);

/** A console write (save, delete, propose, adopt…) the dōjō refused or lost. */
export const actionFailedNotice = (error: string | null) =>
	failureNotice("That change didn't go through", error, { remedy: 'Check what it says, then try again.' });

/** The org overview's "needs a maintainer" band could not be read. */
export const needsUnreadNotice = (error: string | null) => failureNotice("Couldn't load what needs a maintainer", error);

/** An ask's answer could not be sent. */
export const askSendNotice = (error: string | null) =>
	failureNotice("Couldn't send your answer", error, { remedy: 'Send it again. If it keeps failing, answer from the session instead.' });

/** The sign-in card's message: a failure is a fault; the magic-link note waits. */
export function signInNotice(message: string | null, failed: boolean) {
	if (!message) return null;
	return failed
		? failureNotice("Couldn't sign in", message, { remedy: 'Try again. If it keeps failing, use another way to sign in.' })
		: { tone: 'normal' as const, title: message };
}

/** A relay run that could not be read: a fault with Retry — never an empty
 *  state. With its status, a 403 reads as a refusal (M3). */
export function runUnreadNotice(error: string | null, retry?: () => void, failure?: ReadError | null) {
	if (!error && !failure) return null;
	return {
		...loadFailedNotice('', failure ?? error, { service: 'the dōjō' }),
		title: 'Live relay is unavailable',
		...(retry ? { primary: { label: 'Retry', onclick: retry, name: 'retry' } } : {})
	};
}

/** A relay run that is not among the caller's dōjōs: an honest empty, and the way back. */
export const RUN_NOT_FOUND = {
	kanji: '観',
	title: 'This run is not in your dōjōs.',
	description: 'It may have finished long ago, or belong to a dōjō you are not in.',
	action: { label: 'Back to the inbox', href: '/you', name: 'back' }
};

/** A section read that failed, named by what it was. With its status (a
 *  ReadError from the loader) a 401/403 reads as a refusal that says who to ask.
 *  Never null: the screen renders it only where a read failed, and one that gave
 *  no answer still says so, with a next step (B1, r2). */
export function dojoLoadNotice(what: string, error: ReadError | string | null | undefined) {
	return loadFailedNotice(what, error ?? null, { service: 'the dōjō' });
}

/** A route error's way forward: a 401 signs in; anything else goes home. */
export function routeErrorPrimary(status: number) {
	return status === 401
		? { label: 'Sign in', href: '/signin', name: 'signin' }
		: { label: 'Go to your dōjōs', href: '/', name: 'home' };
}
