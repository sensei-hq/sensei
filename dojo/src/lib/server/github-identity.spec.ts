// A GitHub token is trusted only as the GitHub account linked to the caller's
// login (#308 R5a security review). The linked account is Supabase Auth's: the
// `github` identity GoTrue recorded when that login signed in with GitHub —
// `identities[].id`, GitHub's numeric user id. A token is that account's only if
// GitHub's own `GET /user` with it answers the same id. Anything else — another
// account, no linked account, an unreadable answer — is refused, never assumed.
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import { ForgeReadError } from './forge-github';
import { fakeFetch } from './fake-forge-fetch';
import { githubIdentityId, assertGithubAccount, requireGithubTokenOwner } from './github-identity';

const linked = (id: unknown) => ({
	id: 'auth-alice',
	identities: [
		{ provider: 'email', id: 'auth-alice' },
		{ provider: 'github', id, identity_id: '6a2dcde7-551e-4f7f-bc98-aa927e19b421' }
	]
});

describe('githubIdentityId', () => {
	it('is the GitHub user id of the login`s linked github identity', () => {
		expect(githubIdentityId(linked('111'))).toBe('111');
		expect(githubIdentityId(linked(111))).toBe('111');
	});

	it('is null when the login has no GitHub identity, or one with no id', () => {
		for (const u of [null, undefined, {}, { identities: [] }, { identities: [{ provider: 'email', id: 'x' }] }, linked(''), linked(null)]) {
			expect(githubIdentityId(u)).toBeNull();
		}
	});
});

describe('assertGithubAccount', () => {
	it('passes the linked account and refuses any other, or none, with 403', () => {
		expect(() => assertGithubAccount('111', '111')).not.toThrow();
		for (const [actual, expected] of [
			['222', '111'],
			['111', null],
			['111', '']
		] as const) {
			let err: unknown;
			try {
				assertGithubAccount(actual, expected);
			} catch (e) {
				err = e;
			}
			expect(err).toBeInstanceOf(AdminError);
			expect((err as AdminError).status).toBe(403);
		}
	});
});

describe('requireGithubTokenOwner', () => {
	const user = (id: number) => fakeFetch({ '/user': { body: { id, login: 'someone' } } });

	it('accepts a token GitHub says is the linked account', async () => {
		const f = user(111);
		await expect(requireGithubTokenOwner('tok', '111', f.fn)).resolves.toMatchObject({ id: '111' });
		expect(f.calls[0].url).toBe('https://api.github.com/user');
	});

	it('refuses a token of another GitHub account', async () => {
		await expect(requireGithubTokenOwner('tok', '111', user(222).fn)).rejects.toMatchObject({ status: 403 });
	});

	it('refuses when the login has no GitHub account — without asking GitHub', async () => {
		const f = user(111);
		await expect(requireGithubTokenOwner('tok', null, f.fn)).rejects.toMatchObject({ status: 403 });
		expect(f.calls).toEqual([]);
	});

	it('throws when GitHub cannot answer — an unread token is not anyone`s', async () => {
		const f = fakeFetch({ '/user': { status: 401, body: {} } });
		await expect(requireGithubTokenOwner('tok', '111', f.fn)).rejects.toBeInstanceOf(ForgeReadError);
	});
});
