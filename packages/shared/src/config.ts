import { readFile, access } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import os from "os";

export interface SenseiRepoConfig {
  repo_id: string;
  supabase_url: string;
}

export interface SenseiCredentials {
  supabase_service_key: string;
}

/** Read .sensei/config.yaml from repoPath. Returns null if missing. */
export async function loadSenseiConfig(repoPath: string): Promise<SenseiRepoConfig | null> {
  const configPath = join(repoPath, ".sensei", "config.yaml");
  try {
    await access(configPath);
    const raw = await readFile(configPath, "utf-8");
    return yaml.load(raw) as SenseiRepoConfig;
  } catch {
    return null;
  }
}

/** Read credentials from ~/.config/sensei/credentials.yaml, or SUPABASE_SERVICE_KEY env. */
export async function loadCredentials(homeDir?: string): Promise<SenseiCredentials | null> {
  if (process.env.SUPABASE_SERVICE_KEY) {
    return { supabase_service_key: process.env.SUPABASE_SERVICE_KEY };
  }
  const home = homeDir ?? os.homedir();
  const credPath = join(home, ".config", "sensei", "credentials.yaml");
  try {
    await access(credPath);
    const raw = await readFile(credPath, "utf-8");
    return yaml.load(raw) as SenseiCredentials;
  } catch {
    return null;
  }
}
