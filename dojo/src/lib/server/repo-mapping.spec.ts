// repo_key → (provider, org). Spec §II.6, §VIII.5.
//
// The input is ALREADY normalised: the daemon derives `repo_key` with
// `crates/senseid/src/db/pg_store/repo_key.rs::normalize_repo_key`, which
// collapses SSH/HTTPS/ssh:///git://, strips userinfo, port and `.git`, and
// lowercases — so this does NOT re-implement URL parsing. It maps a
// `host/path` key onto the forge that host belongs to and the org within it.
import { describe, it, expect } from 'vitest';
import { forgeRefFromRepoKey } from './repo-mapping';

describe('forgeRefFromRepoKey', () => {
	it('reads the org from the first path segment on github, gitlab and bitbucket', () => {
		expect(forgeRefFromRepoKey('github.com/acme/api')).toEqual({
			provider: 'github',
			org: 'acme'
		});
		expect(forgeRefFromRepoKey('bitbucket.org/acme/api')).toEqual({
			provider: 'bitbucket',
			org: 'acme'
		});
		// GitLab nests subgroups arbitrarily deep; the TENANT is the top-level
		// group, so everything after it is part of the repo path (§II.6).
		expect(forgeRefFromRepoKey('gitlab.com/acme/sub/deeper/api')).toEqual({
			provider: 'gitlab',
			org: 'acme'
		});
	});

	it('handles BOTH Azure DevOps shapes, whose org is at different depths', () => {
		// This pair is the whole reason this is a typed per-provider mapping and
		// not `split('/')[1]`.
		expect(forgeRefFromRepoKey('dev.azure.com/acme/proj/_git/api')).toEqual({
			provider: 'azure_devops',
			org: 'acme'
		});
		// The legacy/SSH form puts a `v3` routing segment first, so the org is
		// SECOND. Taking segment 1 here would map every Azure repo to an org
		// called "v3" — one tenant swallowing every customer's repositories.
		expect(forgeRefFromRepoKey('vs-ssh.visualstudio.com/v3/acme/proj/api')).toEqual({
			provider: 'azure_devops',
			org: 'acme'
		});
		expect(forgeRefFromRepoKey('ssh.dev.azure.com/v3/acme/proj/api')).toEqual({
			provider: 'azure_devops',
			org: 'acme'
		});
		expect(forgeRefFromRepoKey('acme.visualstudio.com/proj/_git/api')).toEqual({
			provider: 'azure_devops',
			org: 'acme'
		});
	});

	it('returns null for a host it does not recognise', () => {
		// Self-hosted GitLab or GitHub Enterprise. Guessing the provider from a
		// path shape would attach an employer's private repo to whichever tenant
		// happened to hold that slug — the repo stays UNMAPPED instead, which
		// §II.6 is explicit about: unmapped, not personal.
		expect(forgeRefFromRepoKey('git.internal.acme.com/acme/api')).toBeNull();
		expect(forgeRefFromRepoKey('gitlab.acme.com/acme/api')).toBeNull();
	});

	it('returns null when there is no org and repo to read', () => {
		expect(forgeRefFromRepoKey('github.com')).toBeNull();
		expect(forgeRefFromRepoKey('github.com/lonely')).toBeNull();
		expect(forgeRefFromRepoKey('')).toBeNull();
		expect(forgeRefFromRepoKey('   ')).toBeNull();
		// v3 with nothing after it
		expect(forgeRefFromRepoKey('ssh.dev.azure.com/v3')).toBeNull();
		expect(forgeRefFromRepoKey('ssh.dev.azure.com/v3/acme')).toBeNull();
	});

	it('is case- and whitespace-insensitive, matching the normaliser', () => {
		// normalize_repo_key lowercases, but a hand-entered key should not map
		// differently from a derived one.
		expect(forgeRefFromRepoKey('  GitHub.com/Acme/API  ')).toEqual({
			provider: 'github',
			org: 'acme'
		});
	});

	it('ignores a leading slash or a trailing one', () => {
		expect(forgeRefFromRepoKey('github.com/acme/api/')).toEqual({
			provider: 'github',
			org: 'acme'
		});
	});

	it('does not treat a host that merely ends in a known one as that forge', () => {
		// `evilgithub.com` and `github.com.attacker.net` are not GitHub. A
		// suffix match would let an attacker-controlled host claim an org slug in
		// a real tenant.
		expect(forgeRefFromRepoKey('evilgithub.com/acme/api')).toBeNull();
		expect(forgeRefFromRepoKey('github.com.attacker.net/acme/api')).toBeNull();
	});
});
