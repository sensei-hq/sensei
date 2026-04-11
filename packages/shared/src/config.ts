import { readFile, access } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import { z } from "zod";
import type { LibEntry } from "./types.js";

const LibEntrySchema = z.object({
  name: z.string(),
  source_type: z.enum(['llms.txt', 'http', 'local', 'github']),
  base_url: z.string().optional(),
  local_path: z.string().optional(),  // legacy — coerced to base_url on load
  description: z.string().optional(),
});

export interface SenseiRepoConfig {
  repo_id: string;
  custom_libs?: LibEntry[];
}

/** Read .sensei/config.yaml from repoPath. Returns null if missing. */
export async function loadSenseiConfig(repoPath: string): Promise<SenseiRepoConfig | null> {
  const configPath = join(repoPath, ".sensei", "config.yaml");
  try {
    await access(configPath);
    const raw = await readFile(configPath, "utf-8");
    const parsed = yaml.load(raw) as Record<string, unknown>;

    let custom_libs: LibEntry[] | undefined;
    if (Array.isArray(parsed?.custom_libs)) {
      const raw = z.array(LibEntrySchema).parse(parsed.custom_libs);
      custom_libs = raw.map(entry => {
        // Coerce legacy local_path to file:// base_url
        if (!entry.base_url && entry.local_path) {
          const fileUrl = entry.local_path.startsWith('/')
            ? `file://${entry.local_path}`
            : `file:///${entry.local_path}`;
          return { ...entry, base_url: fileUrl, local_path: undefined } as LibEntry;
        }
        return entry as LibEntry;
      });
    }

    return { ...(parsed as unknown as SenseiRepoConfig), custom_libs };
  } catch (err) {
    if (err instanceof z.ZodError) throw err;
    return null;
  }
}

