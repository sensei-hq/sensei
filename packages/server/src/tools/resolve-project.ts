import { lookupRepoId, loadSenseiConfig } from "@sensei/shared";

/**
 * Resolve the project name for graph queries.
 * Priority: central registry > .sensei/config.yaml repo_id > fallback repoId
 */
export async function resolveProject(repoPath: string, repoId: string): Promise<string> {
  // Central registry is authoritative
  const registryId = await lookupRepoId(repoPath);
  if (registryId) return registryId;

  // Fallback: per-repo config (legacy)
  const config = await loadSenseiConfig(repoPath).catch(() => null);
  return config?.repo_id ?? repoId;
}
