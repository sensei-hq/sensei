import { loadSenseiConfig } from "@sensei/shared";

/**
 * Resolve the project name for graph queries.
 * Priority: explicit override > .sensei/config.yaml repo_id > fallback repoId
 */
export async function resolveProject(repoPath: string, repoId: string): Promise<string> {
  const config = await loadSenseiConfig(repoPath);
  return config?.repo_id ?? repoId;
}
