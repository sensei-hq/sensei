// "Connect GitHub" on Admin → Integrations (#308 R5a): the dōjō's own GitHub
// sign-in, run again through kavach — the configured `github` provider with the
// scopes kavach.config gives it (read:org among them), redirecting back to the
// screen that asked.
//
// The SAME scopes as sign-in, deliberately: the returning session also runs
// provisioning (provision-on-signin.ts), whose repository-role mirror clears the
// roles on every repository the token cannot list — a narrower re-authorization
// would revoke access, not just read less.
//
// After the redirect, kavach syncs the session to /auth/session; the server
// seals the provider token into its own cookie there (forge-token-cookie.ts).
// Nothing here ever sees the token.

/** The kavach browser instance, narrowed to what this calls. */
export interface KavachSignIn {
	signIn?: (credentials: Record<string, unknown>) => Promise<{ error?: { message?: string } | null } | unknown>;
}

/** A provider as `$kavach/providers` lists it. */
export interface KavachProvider {
	name: string;
	mode?: string;
	scopes?: string[];
}

export async function connectGithub(
	kavach: KavachSignIn,
	providers: KavachProvider[],
	returnTo: string
): Promise<void> {
	if (typeof kavach.signIn !== 'function') throw new Error('sign-in is not ready yet — try again in a moment');
	const github = providers.find((p) => p.name === 'github' && p.mode === 'oauth');
	if (!github) throw new Error('this dōjō has no GitHub sign-in configured');
	const result = (await kavach.signIn({ provider: 'github', scopes: github.scopes ?? [], redirectTo: returnTo })) as {
		error?: { message?: string } | null;
	} | null;
	if (result?.error) throw new Error(result.error.message || 'GitHub sign-in failed');
}
