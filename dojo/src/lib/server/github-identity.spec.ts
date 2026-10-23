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
import { githubIdentityId, assertGithubAccount, requireGithubTokenOwner, isGithubToken } from './github-identity';

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

// A Microsoft sign-in (#308 R5c) also hands the dōjō a provider token — a
// Microsoft Graph token. It must never be sent to GitHub: a login linked to both
// GitHub and Microsoft would otherwise post its Microsoft token to api.github.com
// and read GitHub's 401 as "your GitHub grant is dead". GitHub's own tokens carry
// a documented prefix (gho_ OAuth, ghu_ user-to-server, ghp_/github_pat_ personal,
// ghs_/ghr_ — github.blog "Behind GitHub's new authentication token formats").
describe('isGithubToken', () => {
	it('is true for the documented GitHub token formats', () => {
		for (const t of ['gho_16C7e42F292c6912E7710c838347Ae178B4a', 'ghu_abc123', 'ghp_abc123', 'ghs_abc', 'ghr_abc', 'github_pat_11ABC_def']) {
			expect(isGithubToken(t)).toBe(true);
		}
	});

	it('is false for a Microsoft token (a JWT), anything else, or nothing', () => {
		const jwt = ['eyJ0eXAiOiJKV1QifQ', 'eyJhdWQiOiJ4In0', 'c2ln'].join('.');
		for (const t of [jwt, 'EwBwA8l6BAAU', 'gho_', 'gho_a.b', ' gho_abc', '', null, undefined, 42]) {
			expect(isGithubToken(t)).toBe(false);
		}
	});
});
