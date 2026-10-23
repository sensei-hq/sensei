// Which forge org a repository belongs to, from its normalised key.
//
// The daemon already derives `repo_key` with
// `crates/senseid/src/db/pg_store/repo_key.rs::normalize_repo_key` — SSH,
// HTTPS, `ssh://`, `git://`, userinfo, port and `.git` all collapse to a single
// lowercase `host/org/repo`. Re-implementing that here would be a second
// normaliser to keep in step, so this deliberately does NOT parse URLs: it takes
// the key the daemon computed and answers the narrower question of which forge
// that host is and which segment holds the org (spec §VIII.5).
//
// A host we do not recognise yields `null`, and the repository stays UNMAPPED.
// §II.6 is explicit that unmapped is not personal: defaulting an unknown remote
// to the caller's personal dōjō would silently move an employer's private
// repository into a free personal tenant.
import type { ForgeProvider } from './forge-github';

/** The forge coordinates a repository key resolves to. */
export interface ForgeRef {
	provider: ForgeProvider;
	/** The forge's name for the org — matched against
	 *  `tenant_connections.external_slug`. */
	org: string;
	/** Azure DevOps only: the project the repository is in, decoded and
	 *  lowercased — an Azure project is a workspace of its organization's dōjō
	 *  (#308 R5b, D36), mapped as `{org}/{project}`. Absent when the key does not
	 *  name one readably; the org still maps. */
	project?: string;
}

/**
 * How to find the org for one forge host.
 *
 * `orgIndex` is which path segment holds it, and it is NOT always the first —
 * Azure's SSH/legacy form routes through a `v3` segment, so its org is second.
 * Reading segment 1 there would map every Azure repository to an org literally
 * called "v3", collecting every customer's repos into one tenant.
 */
interface HostRule {
	provider: ForgeProvider;
	/** Path segment holding the org (0-based, after the host). */
	orgIndex: number;
	/** Segments the path must have at least, so `org/repo` is really present. */
	minSegments: number;
}

/** Exact hosts only. A suffix match would let `github.com.attacker.net` or
 *  `evilgithub.com` claim an org slug inside a real tenant. */
const HOSTS: Record<string, HostRule> = {
	'github.com': { provider: 'github', orgIndex: 0, minSegments: 2 },
	'gitlab.com': { provider: 'gitlab', orgIndex: 0, minSegments: 2 },
	'bitbucket.org': { provider: 'bitbucket', orgIndex: 0, minSegments: 2 },
	// https://dev.azure.com/{org}/{project}/_git/{repo}
	'dev.azure.com': { provider: 'azure_devops', orgIndex: 0, minSegments: 2 },
	// ssh://ssh.dev.azure.com/v3/{org}/{project}/{repo}
	'ssh.dev.azure.com': { provider: 'azure_devops', orgIndex: 1, minSegments: 3 },
	// {org}@vs-ssh.visualstudio.com:v3/{org}/{project}/{repo}
	'vs-ssh.visualstudio.com': { provider: 'azure_devops', orgIndex: 1, minSegments: 3 }
};

/** `{org}.visualstudio.com/{project}/_git/{repo}` — the org is in the HOST, not
 *  the path, which no segment index can express. */
const VISUALSTUDIO_SUFFIX = '.visualstudio.com';

/** A project segment, percent-decoded (`mobile%20apps` → `mobile apps`), or
 *  undefined when it does not decode or decodes to something that is not one
 *  name (an Azure project name holds no `/`). */
function projectName(segment: string | undefined): string | undefined {
	if (!segment) return undefined;
	let name: string;
	try {
		name = decodeURIComponent(segment);
	} catch {
		return undefined;
	}
	return name && !name.includes('/') && name !== '_git' ? name : undefined;
}

/**
 * The project an Azure DevOps repository path names, given the path after the
 * org: `{project}/_git/{repo}` (https), `_git/{repo}` (the repository named after
 * its project), or `{project}/{repo}` (SSH, `sshForm`).
 */
function azureProject(rest: string[], sshForm: boolean): string | undefined {
	if (sshForm) return rest.length >= 2 ? projectName(rest[0]) : undefined;
	if (rest[0] === '_git') return projectName(rest[1]);
	return rest[1] === '_git' ? projectName(rest[0]) : undefined;
}

/** The ref, with the Azure project when one is readable. */
function azureRef(org: string, project: string | undefined): ForgeRef {
	return project ? { provider: 'azure_devops', org, project } : { provider: 'azure_devops', org };
}

/**
 * The forge org a repository key belongs to, or `null` when the host is not one
 * we can attribute. Pure.
 */
export function forgeRefFromRepoKey(repoKey: string): ForgeRef | null {
	const key = (typeof repoKey === 'string' ? repoKey : '').trim().toLowerCase();
	if (!key) return null;

	const parts = key.split('/').filter((p) => p.length > 0);
	if (parts.length < 2) return null;

	const [host, ...path] = parts;

	const rule = HOSTS[host];
	if (rule) {
		if (path.length < rule.minSegments) return null;
		const org = path[rule.orgIndex];
		if (!org) return null;
		if (rule.provider === 'azure_devops') {
			return azureRef(org, azureProject(path.slice(rule.orgIndex + 1), host !== 'dev.azure.com'));
		}
		return { provider: rule.provider, org };
	}

	// The per-account Azure host. Guarded by an exact suffix on a label boundary
	// AND a non-empty prefix, so `visualstudio.com` alone and
	// `x.visualstudio.com.attacker.net` both miss.
	if (host.endsWith(VISUALSTUDIO_SUFFIX)) {
		const org = host.slice(0, -VISUALSTUDIO_SUFFIX.length);
		// `vs-ssh` is the shared SSH endpoint handled above, not an org.
		if (org && !org.includes('.') && path.length >= 2) {
			// The pre-2018 URL carries the collection: `{org}.visualstudio.com/DefaultCollection/{project}/_git/{repo}`.
			const rest = path[0] === 'defaultcollection' ? path.slice(1) : path;
			return azureRef(org, azureProject(rest, false));
		}
		return null;
	}

	return null;
}
