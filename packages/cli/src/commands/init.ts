import { intro, outro, spinner, note, log, isCancel, text, multiselect, confirm } from "@clack/prompts";
import { writeFile, mkdir, access, readFile, chmod } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { createClient } from "@supabase/supabase-js";
import { indexRepo } from "@sensei/engine";
import { installHooks } from "@sensei/collector";
import { scanDirectDeps, inferSourceType } from "../lib/detect-libs.js";
import { installSkills, installSkillsFromCatalog, promptAndInstallSkillsFromCatalog } from "./install-skills.js";
import { runUpdateRegistryCore } from "./update-registry.js";
import { setupMcp } from "./setup.js";
import type { LibEntry } from "@sensei/shared";
import { claudeMdTemplate } from "../templates/claude-md.js";
import { agentsMdTemplate } from "../templates/agents-md.js";

const DEFAULT_SUPABASE_URL = "http://localhost:54321";

export interface InitOptions {
  /** Install skills and hooks globally (~/.claude/) rather than repo-local */
  global?: boolean;
  /** Supabase URL — skips prompt when provided */
  supabaseUrl?: string;
  /** Supabase service role key — skips prompt when provided */
  serviceKey?: string;
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

/** Looks up a lib by name in the global shared pool. Returns catalog row or null. */
export async function lookupSharedLib(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  client: any,
  name: string,
): Promise<{
  id: string;
  section_count: number;
  indexed_at: string;
  base_url: string | null;
  local_path: string | null;
  source_type: string;
} | null> {
  try {
    const { data } = await (client as any)
      .schema('sensei')
      .from('shared_libs')
      .select('id,section_count,indexed_at,base_url,local_path,source_type')
      .eq('name', name)
      .maybeSingle();
    return data ?? null;
  } catch {
    return null;
  }
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

  // 3. Resolve Supabase credentials
  //    Priority: CLI flag / env var → prompt
  let supabaseUrl = opts.supabaseUrl ?? "";
  let serviceKey = opts.serviceKey ?? "";

  if (!supabaseUrl) {
    const val = await text({
      message: "Supabase URL:",
      placeholder: DEFAULT_SUPABASE_URL,
      defaultValue: DEFAULT_SUPABASE_URL,
      validate: v => (v.startsWith("http") ? undefined : "Must be a URL"),
    });
    if (isCancel(val)) { outro("Cancelled."); return; }
    supabaseUrl = String(val) || DEFAULT_SUPABASE_URL;
  }

  if (!serviceKey) {
    const val = await text({
      message: "Supabase service role key:",
      validate: v => (v.length > 10 ? undefined : "Looks too short"),
    });
    if (isCancel(val)) { outro("Cancelled."); return; }
    serviceKey = String(val);
  }

  // Create Supabase client
  const client = createClient(supabaseUrl, serviceKey, {
    db: { schema: "sensei" },
    auth: { persistSession: false },
  });

  const repoName = cwd.split("/").pop() ?? "repo";
  const { data: repo, error: repoErr } = await client.from("repos").upsert({
    name: repoName,
    local_path: cwd,
    stack,
    entry_points: entryPoints,
  }, { onConflict: "local_path" }).select("id").single();

  if (repoErr || !repo) {
    log.error(`Failed to register repo: ${repoErr?.message ?? "no data returned"}`);
    outro("Failed."); return;
  }
  const repoId: string = repo.id;

  // Pass 2: For each candidate, check shared pool first; else prompt for URL
  const customLibs: LibEntry[] = [];
  const linkedLibEntries: LibEntry[] = [];

  if (candidates.length > 0) {
    for (const name of candidates) {
      const sharedLib = await lookupSharedLib(client, name);

      if (sharedLib) {
        const daysAgo = Math.floor((Date.now() - new Date(sharedLib.indexed_at).getTime()) / 86400000);
        const confirmed = await confirm({
          message: `${name} is already indexed globally (${sharedLib.section_count} sections, ${daysAgo}d ago). Link it?`,
          initialValue: true,
        });
        if (!isCancel(confirmed) && confirmed) {
          const baseUrl = sharedLib.base_url
            ?? (sharedLib.local_path ? `file://${sharedLib.local_path}` : "");
          linkedLibEntries.push({
            name,
            source_type: sharedLib.source_type as LibEntry["source_type"],
            base_url: baseUrl,
          });
          try {
            await (client as any).schema('sensei').from('repo_libs').upsert({
              repo_id: repoId,
              name,
              source_type: sharedLib.source_type,
              base_url: baseUrl,
              shared_lib_id: sharedLib.id,
            }, { onConflict: 'repo_id,name' });
          } catch {
            log.warn(`  repo_libs upsert failed for ${name} (shared link)`);
          }
          continue;
        }
      }

      const input = await text({
        message: `Docs for "${name}"? (llms.txt URL, HTTP page, raw .md URL, or local path — Enter to skip)`,
        placeholder: "https://example.com/llms.txt",
      });
      if (isCancel(input) || !input?.trim()) continue;
      customLibs.push({ name, ...inferSourceType(String(input).trim()) });
    }
  }

  // 4. Write .sensei/config.yaml and credentials
  const senseiDir = join(cwd, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  const allConfigLibs = [...customLibs, ...linkedLibEntries];
  const customLibsYaml = allConfigLibs.length > 0
    ? `custom_libs:\n${allConfigLibs.map(l => {
        return `  - name: ${l.name}\n    source_type: ${l.source_type}\n    base_url: ${l.base_url}`;
      }).join("\n")}\n`
    : "";
  await writeFile(
    join(senseiDir, "config.yaml"),
    `repo_id: ${repoId}\nsupabase_url: ${supabaseUrl}\n${customLibsYaml}`,
  );

  // Write credentials to ~/.config/sensei/ (global, not committed)
  const credsDir = join(homedir(), ".config", "sensei");
  await mkdir(credsDir, { recursive: true });
  const credsPath = join(credsDir, "credentials.yaml");
  await writeFile(credsPath, `supabase_service_key: ${serviceKey}\n`);
  await chmod(credsPath, 0o600);

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
  let result;
  try {
    result = await indexRepo({ repoPath: cwd, repoId, client: client as any });
    indexSpinner.stop(`Indexed: ${result.filesIndexed} files, ${result.symbolsUpserted} symbols`);
    if (result.errors.length > 0) {
      log.warn(`${result.errors.length} indexing errors — see details below`);
      result.errors.slice(0, 3).forEach((e: string) => log.warn(e));
    }
    await client.from("repos").update({ last_indexed_at: new Date().toISOString() }).eq("id", repoId);
  } catch (err) {
    indexSpinner.stop(`Indexing failed: ${err instanceof Error ? err.message : String(err)}`);
    log.warn("You can re-index later with: sensei index");
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
