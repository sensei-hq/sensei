// packages/cli/src/commands/update-registry.ts
import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { intro, outro, log, spinner } from "@clack/prompts";
import {
  extractProjectProfile,
  LibIndexer,
  LibSkillGenerator,
  SkillValidator,
  ClaudeAdapter,
  LlmsTxtAdapter,
  HttpAdapter,
  LocalAdapter,
  type SourceAdapter,
} from "@sensei/engine";
import { ClaudeBackend, OllamaBackend } from "@sensei/server";
import { makeSenseiClient, loadSenseiConfig, type LibEntry, type LibSkillFile, type LibSkillsManifest } from "@sensei/shared";

function createAdapter(sourceType: LibEntry["source_type"]): SourceAdapter {
  if (sourceType === "llms.txt") return new LlmsTxtAdapter();
  if (sourceType === "http") return new HttpAdapter();
  return new LocalAdapter();
}

/** Core logic without clack UI. Called directly from init (no libName arg) or via updateRegistry. When libName is provided and the lib is not found, exits non-zero. */
export async function runUpdateRegistryCore(repoPath: string, libName?: string, opts?: { global?: boolean }): Promise<number> {
  const config = await loadSenseiConfig(repoPath);
  if (!config) {
    log.error("Not initialised — run sensei init first");
    if (libName || opts?.global) process.exit(1);
    return 0;
  }

  if (!config.custom_libs?.length) {
    if (libName) {
      log.error("No custom_libs in config — add entries first");
      process.exit(1);
    }
    log.info("No custom_libs configured in .sensei/config.yaml");
    return 0;
  }

  const libs = libName
    ? config.custom_libs.filter(l => l.name === libName)
    : config.custom_libs;

  if (libName && libs.length === 0) {
    log.error(`Library '${libName}' not found in custom_libs`);
    process.exit(1);
  }

  const client = await makeSenseiClient(repoPath);
  if (!client) {
    log.error("Supabase client not configured. Run sensei init first.");
    return 0;
  }

  const repoId = config.repo_id;

  const profileSpinner = spinner();
  profileSpinner.start("Analysing project...");
  const profile = await extractProjectProfile(client as any, repoId, repoPath);
  profileSpinner.stop(`Project analysed: ${profile.dominantLanguage}`);

  const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const ollamaBackend = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });

  const manifestPath = join(repoPath, ".sensei", "lib-skills.json");
  let manifest: LibSkillsManifest = { repoSlug, skills: [], updatedAt: new Date().toISOString() };
  try {
    const raw = await readFile(manifestPath, "utf-8");
    manifest = JSON.parse(raw) as LibSkillsManifest;
  } catch { /* start fresh */ }

  const hasAnthropicKey = Boolean(process.env.ANTHROPIC_API_KEY);
  let claudeBackend: ClaudeBackend | null = null;

  for (const lib of libs) {
    const fetchSpin = spinner();
    fetchSpin.start(`Fetching ${lib.name}...`);
    let pages;
    try {
      pages = await createAdapter(lib.source_type).fetch(lib);
      fetchSpin.stop(`Fetched ${lib.name}: ${pages.length} pages`);
    } catch (err) {
      fetchSpin.stop(`Error fetching ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }

    const indexSpin = spinner();
    indexSpin.start(`Indexing ${lib.name} (${pages.length} pages)...`);
    let sectionsIndexed = 0;
    let sharedLibId: string | null = null;

    try {
      if (opts?.global) {
        // Upsert shared_libs catalog
        const { data: sharedLib, error: sharedLibErr } = await (client as any)
          .schema('sensei')
          .from('shared_libs')
          .upsert(
            { name: lib.name, source_type: lib.source_type, base_url: lib.base_url ?? null, local_path: lib.local_path ?? null },
            { onConflict: 'name' }
          )
          .select('id')
          .single();

        if (sharedLibErr || !sharedLib) throw new Error(`shared_libs upsert failed: ${sharedLibErr?.message}`);
        sharedLibId = sharedLib.id;

        // Index into shared pool
        const result = await new LibIndexer(client as any, ollamaBackend).indexShared(sharedLibId!, lib, pages);
        sectionsIndexed = result.sectionsIndexed;
        indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed (shared pool)`);

        // Update catalog counts
        await (client as any).schema('sensei').from('shared_libs')
          .update({ section_count: sectionsIndexed, indexed_at: new Date().toISOString() })
          .eq('id', sharedLibId);
      } else {
        const result = await new LibIndexer(client as any, ollamaBackend).index(repoId, lib, pages);
        sectionsIndexed = result.sectionsIndexed;
        indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed`);
      }
    } catch (err) {
      indexSpin.stop(`Error indexing ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }

    let libSkillFile: LibSkillFile | undefined;
    if (hasAnthropicKey) {
      const skillSpin = spinner();
      skillSpin.start(`Generating skill for ${lib.name}...`);
      try {
        if (!claudeBackend) {
          claudeBackend = new ClaudeBackend();
          await claudeBackend.init();
        }
        const validator = new SkillValidator(claudeBackend, profile);
        const markdown = await new LibSkillGenerator(claudeBackend, profile, validator).generate(lib, pages);
        libSkillFile = await new ClaudeAdapter().writeLibSkill(lib.name, markdown, repoSlug);

        manifest.skills = manifest.skills.filter(s => s.libName !== lib.name);
        manifest.skills.push(libSkillFile);
        manifest.updatedAt = new Date().toISOString();
        await mkdir(join(repoPath, ".sensei"), { recursive: true });
        await writeFile(manifestPath, JSON.stringify(manifest, null, 2), "utf-8");

        skillSpin.stop(`Skill written: ${libSkillFile.path}`);
      } catch (err) {
        skillSpin.stop(`Skill generation skipped for ${lib.name}`);
        log.warn(`  ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    // Sync to repo_libs so dashboard can read config without local file access.
    // Skill fields only included when generation succeeded — avoids clobbering a good skill_path.
    try {
      await (client as any)
        .schema('sensei')
        .from('repo_libs')
        .upsert({
          repo_id: repoId,
          name: lib.name,
          source_type: lib.source_type,
          base_url: lib.base_url ?? null,
          local_path: lib.local_path ?? null,
          ...(sharedLibId ? { shared_lib_id: sharedLibId } : {}),
          ...(libSkillFile ? { skill_path: libSkillFile.path, skill_generated_at: libSkillFile.generatedAt } : {}),
        }, { onConflict: 'repo_id,name' });
    } catch (err) {
      log.warn(`  repo_libs upsert failed for ${lib.name}: ${err instanceof Error ? err.message : String(err)}`);
    }
    // ← upsert closes here; next line is the closing `}` of the for loop
  }

  return libs.length;
}

/** Full command with clack UI — called from CLI. */
export async function updateRegistry(repoPath: string, libName?: string, opts?: { global?: boolean }): Promise<void> {
  intro("sensei update-registry");
  const count = await runUpdateRegistryCore(repoPath, libName, opts);
  outro(`Done. ${count} librar${count === 1 ? "y" : "ies"} processed.`);
}
