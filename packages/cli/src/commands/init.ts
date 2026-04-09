import { intro, outro, spinner, note, log, isCancel, text, multiselect } from "@clack/prompts";
import { writeFile, mkdir, access, readFile } from "fs/promises";
import { join } from "path";
import { randomUUID } from "crypto";
import { installHooks } from "@sensei/collector";
import { scanDirectDeps, inferSourceType } from "../lib/detect-libs.js";
import { installSkills, installSkillsFromCatalog, promptAndInstallSkillsFromCatalog } from "./install-skills.js";
import { runUpdateRegistryCore } from "./update-registry.js";
import { setupMcp } from "./setup.js";
import type { LibEntry } from "@sensei/shared";
import { claudeMdTemplate } from "../templates/claude-md.js";
import { agentsMdTemplate } from "../templates/agents-md.js";
import { indexRepo } from "@sensei/graph-indexer";

export interface InitOptions {
  /** Install skills and hooks globally (~/.claude/) rather than repo-local */
  global?: boolean;
  /** Install recommended skills without prompting */
  useRecommended?: boolean;
}

async function detectUnknownLibs(deps: string[]): Promise<string[]> {
  if (!deps.length) return [];

  if (process.env.ANTHROPIC_API_KEY) {
    try {
      const { ClaudeBackend } = await import("@sensei/server");
      const backend = new ClaudeBackend();
      await backend.init();
      const prompt = `Given this list of npm/pip/go packages, which are niche, recently released, or not well-covered in your training data? An AI agent using these packages would benefit from having their docs indexed. List only package names, one per line. Be conservative — only flag packages where indexed docs would genuinely help.\n\nPackages:\n${deps.join("\n")}`;
      const response = await backend.generate(prompt);
      return response
        .split("\n")
        .map(l => l.trim().replace(/^(?:[-*•]|\d+[.):]?)\s*/, ""))
        .filter(name => deps.includes(name));
    } catch {
      // Fall through to multiselect
    }
  }
  return [];
}


export async function init(cwd: string, opts: InitOptions = {}): Promise<void> {
  intro("sensei init");

  // 1. Detect stack from manifest files
  const stack: string[] = [];
  const entryPoints: Array<{ path: string; role: string }> = [];

  try {
    const pkgJson = JSON.parse(await readFile(join(cwd, "package.json"), "utf-8").catch(() => "null"));
    if (pkgJson) {
      stack.push("typescript");
      if (pkgJson.dependencies?.["react"] || pkgJson.dependencies?.["svelte"]) {
        stack.push(pkgJson.dependencies?.["react"] ? "react" : "svelte");
      }
      if (pkgJson.main) entryPoints.push({ path: pkgJson.main, role: "main" });
      if (pkgJson.exports?.["."]?.default) entryPoints.push({ path: pkgJson.exports["."].default, role: "main" });
    }
  } catch {}

  try {
    await access(join(cwd, "pyproject.toml"));
    stack.push("python");
  } catch {}

  try {
    await access(join(cwd, "go.mod"));
    stack.push("go");
  } catch {}

  // 2. Scan dependencies for potential custom_libs candidates
  const allDeps = await scanDirectDeps(cwd);
  let candidates: string[] = [];

  if (allDeps.length > 0) {
    const llmCandidates = await detectUnknownLibs(allDeps);
    if (llmCandidates.length > 0) {
      candidates = llmCandidates;
    } else {
      const selected = await multiselect({
        message: "Which libraries would you like to index docs for? (space to select, enter to confirm)",
        options: allDeps.map(d => ({ value: d, label: d })),
        required: false,
      });
      if (isCancel(selected)) {
        candidates = [];
      } else {
        candidates = selected as string[];
      }
    }
  }

  // 3. Generate local repo ID (deterministic per path)
  //    TODO: replace with SQLite-backed repo registry when graph indexer ships
  const repoName = cwd.split("/").pop() ?? "repo";
  const repoId: string = randomUUID();

  // Pass 2: Prompt for doc URLs for each candidate lib
  const customLibs: LibEntry[] = [];

  if (candidates.length > 0) {
    for (const name of candidates) {
      const input = await text({
        message: `Docs for "${name}"? (llms.txt URL, HTTP page, raw .md URL, or local path — Enter to skip)`,
        placeholder: "https://example.com/llms.txt",
      });
      if (isCancel(input) || !input?.trim()) continue;
      customLibs.push({ name, ...inferSourceType(String(input).trim()) });
    }
  }

  // 4. Write .sensei/config.yaml
  const senseiDir = join(cwd, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  const customLibsYaml = customLibs.length > 0
    ? `custom_libs:\n${customLibs.map(l => {
        return `  - name: ${l.name}\n    source_type: ${l.source_type}\n    base_url: ${l.base_url}`;
      }).join("\n")}\n`
    : "";
  await writeFile(
    join(senseiDir, "config.yaml"),
    `repo_id: ${repoId}\n${customLibsYaml}`,
  );

  if (customLibs.length > 0) {
    const libSpin = spinner();
    libSpin.start("Indexing library docs...");
    try {
      await runUpdateRegistryCore(cwd);
      libSpin.stop(`Library docs indexed (${customLibs.length} ${customLibs.length === 1 ? "lib" : "libs"})`);
    } catch (err) {
      libSpin.stop("Library indexing skipped — run sensei update-registry when ready");
      log.warn(`  ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  // 5. Run first index
  const indexSpinner = spinner();
  indexSpinner.start("Indexing repo (first full scan)...");
  try {
    const result = await indexRepo({ repoPath: cwd, repoId, project: repoId });
    indexSpinner.stop(`Indexed: ${result.filesIndexed} files, ${result.functionsIndexed} functions, ${result.edgesCreated} edges (${result.durationMs}ms)`);
  } catch (err) {
    indexSpinner.stop(`Indexing failed: ${err instanceof Error ? err.message : String(err)}`);
    log.warn("You can re-index later with: sensei add");
  }

  // 6. Install hooks
  //    --global → hooks in ~/.claude/settings.json + daemon autostart
  //    default  → hooks in <repo>/.claude/settings.json (no daemon; collector started manually)
  const hookSpinner = spinner();
  hookSpinner.start("Installing collector hooks...");
  try {
    await installHooks();
    hookSpinner.stop(`Hooks installed${opts.global ? " (global)" : ""}`);
  } catch (err) {
    hookSpinner.stop(`Hook install skipped: ${err instanceof Error ? err.message : String(err)}`);
  }

  // 7. Register MCP server in ~/.claude/mcp.json (always — same server for all repos via env var)
  try {
    await setupMcp(cwd);
    log.success("MCP server registered — restart Claude Code to activate");
  } catch (err) {
    log.warn(`MCP registration skipped: ${err instanceof Error ? err.message : String(err)}`);
  }

  // 8. Write CLAUDE.md and AGENTS.md — always local to the repo
  await writeFile(join(cwd, "CLAUDE.md"), claudeMdTemplate({ repoName, stack, repoId }));
  await writeFile(join(cwd, "AGENTS.md"), agentsMdTemplate({ repoName, stack }));

  // 9. Install skills
  if (opts.global) {
    await installSkills(cwd, "global");
  } else if (opts.useRecommended) {
    await installSkillsFromCatalog(cwd, "recommended");
  } else {
    await promptAndInstallSkillsFromCatalog(cwd);
  }

  note(
    [
      `Created: .sensei/config.yaml`,
      ``,
      `Next steps:`,
      `  1. Restart Claude Code to activate the MCP server`,
      `  2. Start the dashboard: cd apps/dashboard && bun run dev`,
      `  3. Start the collector: sensei serve`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
