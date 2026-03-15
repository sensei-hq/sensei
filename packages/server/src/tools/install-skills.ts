import type { SupabaseClient } from "@supabase/supabase-js";
import { writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { extractProjectProfile, SkillGenerator, SkillValidator, ClaudeAdapter } from "@sensei/engine";
import { ClaudeBackend } from "../model/claude-backend.js";
import type { AgentSkillsManifest } from "@sensei/shared";

export async function installSkillsTool(
  db: SupabaseClient,
  repoId: string,
  repoPath: string,
): Promise<{ filesWritten: string[]; errors: string[] }> {
  try {
    const backend = new ClaudeBackend();
    await backend.init();

    const profile = await extractProjectProfile(db, repoId, repoPath);
    const validator = new SkillValidator(backend, profile);
    const generator = new SkillGenerator(backend, profile, validator);
    const skills = await generator.generate();

    const adapter = new ClaudeAdapter();
    const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    const written = await adapter.writeSkills(skills, repoSlug);

    // Write manifest so dashboard reflects updated state
    const manifest: AgentSkillsManifest = {
      agent: "claude",
      repoSlug,
      skills: written,
      updatedAt: new Date().toISOString(),
    };
    const senseiDir = join(repoPath, ".sensei");
    await mkdir(senseiDir, { recursive: true });
    await writeFile(join(senseiDir, "agent-skills.json"), JSON.stringify(manifest, null, 2), "utf-8");

    return { filesWritten: written.map(f => f.path), errors: [] };
  } catch (err) {
    return { filesWritten: [], errors: [err instanceof Error ? err.message : String(err)] };
  }
}
