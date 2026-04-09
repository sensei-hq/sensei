// packages/cli/src/commands/update-registry.ts
import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { intro, outro, log, spinner } from "@clack/prompts";
import {
  extractProjectProfile,
  LibSkillGenerator,
  SkillValidator,
  ClaudeAdapter,
  LlmsTxtAdapter,
  HttpAdapter,
  LocalAdapter,
  type SourceAdapter,
} from "@sensei/engine";
import { ClaudeBackend, LibIndexer } from "@sensei/server";
import { loadSenseiConfig, type LibEntry, type LibSkillFile, type LibSkillsManifest } from "@sensei/shared";

function createAdapter(sourceType: LibEntry["source_type"]): SourceAdapter {
  if (sourceType === "llms.txt") return new LlmsTxtAdapter();
  if (sourceType === "http") return new HttpAdapter();
  return new LocalAdapter();
}

/** Core logic without clack UI. Called directly from init (no libName arg) or via updateRegistry. When libName is provided and the lib is not found, exits non-zero. */
export async function runUpdateRegistryCore(repoPath: string, libName?: string): Promise<number> {
  const config = await loadSenseiConfig(repoPath);
  if (!config) {
    log.error("Not initialised — run sensei init first");
    if (libName) process.exit(1);
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

  const repoId = config.repo_id;

  const profileSpinner = spinner();
  profileSpinner.start("Analysing project...");
  const profile = await extractProjectProfile(repoId, repoPath);
  profileSpinner.stop(`Project analysed: ${profile.dominantLanguage}`);

  const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");

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

    try {
      const result = await new LibIndexer(repoId).index(lib, pages);
      sectionsIndexed = result.sectionsIndexed;
      indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed`);
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
  }

  return libs.length;
}

/** Full command with clack UI — called from CLI. */
export async function updateRegistry(repoPath: string, libName?: string): Promise<void> {
  intro("sensei update-registry");
  const count = await runUpdateRegistryCore(repoPath, libName);
  outro(`Done. ${count} librar${count === 1 ? "y" : "ies"} processed.`);
}
