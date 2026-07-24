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

	it('GitHub button is enabled and starts GitHub OAuth via kavach on click', async () => {
		const calls: Array<{ provider: string; redirectTo?: string }> = [];
		const signIn = async (c: { provider: string; redirectTo?: string }) => {
			calls.push(c);
			return {};
		};
		const { getByRole } = render(DojoSignIn, { context: new Map([['kavach', { signIn }]]) });
		const btn = getByRole('button', { name: /Continue with GitHub/ }) as HTMLButtonElement;
		expect(btn.disabled).toBe(false);
		btn.click();
		await Promise.resolve();
		expect(calls).toHaveLength(1);
		expect(calls[0].provider).toBe('github');
	});
});
