import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoSignIn from './DojoSignIn.svelte';

// DojoSignIn renders the marketing chrome + one kavach AuthProvider per configured
// provider (github OAuth · magic-link). The sign-in mechanics + scopes live in
// kavach's AuthProvider (stubbed here — see vitest.config.ts), not this screen, so
// these specs assert what DojoSignIn owns: the heading, the provider options, and
// the insight cards. They render with NO kavach context (static-render safe).
describe('DojoSignIn', () => {
	afterEach(cleanup);

	it('renders the heading and one option per configured provider', () => {
		const { getByText } = render(DojoSignIn);
		expect(getByText('Sign in to continue')).toBeTruthy();
		// Both providers from the config render (via AuthProvider).
		expect(getByText('Continue with GitHub')).toBeTruthy();
		expect(getByText('Email me a magic link')).toBeTruthy();
	});

	// #132 / #241: the left pane is the mockup's product copy (p-signin). It
	// showed "Acme Corp's" lesson counts and a "+11pp" lift from a fixture — to
	// every visitor, before sign-in, as fact.
	it('describes the product, as the mockup does — no invented statistics', () => {
		const { getByText, container } = render(DojoSignIn);
		expect(container.textContent).toContain('A quiet companion');
		for (const card of ['Watches your sessions', 'Your rules & guardrails', 'Nothing leaves']) {
			expect(getByText(card)).toBeTruthy();
		}
		expect(getByText(/Sign in and sensei picks up where you left off/)).toBeTruthy();
		expect(container.textContent).not.toMatch(/Acme Corp|lessons shared this week|first-try resolution/);
	});

	it('offers the self-hosted-Dōjō entry point', () => {
		const { getByText } = render(DojoSignIn);
		expect(getByText(/Connecting to a self-hosted/i)).toBeTruthy();
	});
});
