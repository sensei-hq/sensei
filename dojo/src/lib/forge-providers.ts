// The git providers a dōjō can come from — `dojo.forge_provider`. One list for
// the server (forge reads, provisioning, the Integrations routes) and the screen
// that names them. Pure, so a client module may import it.

export const FORGE_PROVIDERS = ['github', 'gitlab', 'bitbucket', 'azure_devops'] as const;
export type ForgeProvider = (typeof FORGE_PROVIDERS)[number];

/** Whether a value names a `dojo.forge_provider`. */
export function isForgeProvider(v: unknown): v is ForgeProvider {
	return typeof v === 'string' && (FORGE_PROVIDERS as readonly string[]).includes(v);
}
