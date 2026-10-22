import { describe, it, expect, vi } from 'vitest';
import { connectGithub } from './github-connect';

// "Connect GitHub" (#308 R5a) is the dōjō's own GitHub sign-in run again — the
// configured provider, with the scopes kavach.config gives it (read:org among
// them), back to the screen that asked. Never a hand-rolled OAuth URL.
const PROVIDERS = [
	{ name: 'github', mode: 'oauth', label: 'Continue with GitHub', scopes: ['read:org', 'user:email', 'repo'] },
	{ name: 'magic', mode: 'otp', label: 'Email me a magic link' }
];

describe('connectGithub', () => {
	it('signs in with the configured GitHub provider and its scopes, returning to the screen', async () => {
		const signIn = vi.fn().mockResolvedValue({ type: 'success' });
		await connectGithub({ signIn }, PROVIDERS, 'http://127.0.0.1:5173/org/acme/integrations');
		expect(signIn).toHaveBeenCalledWith({
			provider: 'github',
			scopes: ['read:org', 'user:email', 'repo'],
			redirectTo: 'http://127.0.0.1:5173/org/acme/integrations'
		});
	});

	it('says so when the sign-in refused, when kavach is not ready, or GitHub is not configured', async () => {
		await expect(
			connectGithub({ signIn: vi.fn().mockResolvedValue({ error: { message: 'provider is not enabled' } }) }, PROVIDERS, 'x')
		).rejects.toThrow('provider is not enabled');
		await expect(connectGithub({}, PROVIDERS, 'x')).rejects.toThrow(/not ready/);
		await expect(connectGithub({ signIn: vi.fn() }, PROVIDERS.slice(1), 'x')).rejects.toThrow(/GitHub sign-in/);
	});
});
