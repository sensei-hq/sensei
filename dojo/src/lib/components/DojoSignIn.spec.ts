import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoSignIn from './DojoSignIn.svelte';

// Renders the magic-link sign-in with NO kavach context (getContext returns
// undefined) — proving the screen static-renders without a live auth session.
describe('DojoSignIn', () => {
	afterEach(cleanup);

	it('renders the sign-in heading and magic-link form without a kavach session', () => {
		const { getByText, getByPlaceholderText, getByLabelText } = render(DojoSignIn);

		expect(getByText('Sign in to continue')).toBeTruthy();
		// Magic-link is the primary R6 auth path.
		expect(getByText('Email me a magic link')).toBeTruthy();
		expect(getByLabelText('Work email')).toBeTruthy();
		expect(getByPlaceholderText('you@company.com')).toBeTruthy();
	});

	it('renders the welcome-back insight cards from the metrics', () => {
		const { getByText } = render(DojoSignIn);
		expect(getByText('lessons shared this week')).toBeTruthy();
		expect(getByText('approved & distributed')).toBeTruthy();
	});
});
