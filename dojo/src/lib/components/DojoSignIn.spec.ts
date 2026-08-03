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

	it('renders the welcome-back insight cards from the metrics', () => {
		const { getByText } = render(DojoSignIn);
		expect(getByText('lessons shared this week')).toBeTruthy();
		expect(getByText('approved & distributed')).toBeTruthy();
	});

	it('offers the self-hosted-Dōjō entry point', () => {
		const { getByText } = render(DojoSignIn);
		expect(getByText(/Connecting to a self-hosted/i)).toBeTruthy();
	});
});
