import { intro, outro, spinner, note, log, isCancel, text, multiselect } from "@clack/prompts";
import { writeFile, mkdir, access, readFile, chmod } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { createClient } from "@supabase/supabase-js";
import { indexRepo } from "@sensei/engine";
import { claudeMdTemplate } from "../templates/claude-md.js";
import { agentsMdTemplate } from "../templates/agents-md.js";
import { installHooks } from "@sensei/collector";
import { scanDirectDeps, inferSourceType } from "../lib/detect-libs.js";
import { runUpdateRegistryCore } from "./update-registry.js";
import type { LibEntry } from "@sensei/shared";

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
        .map(l => l.trim().replace(/^[-*•]\s*/, ""))
        .filter(name => deps.includes(name));
    } catch {
      // Fall through to multiselect
    }
  }
  return [];
}

export async function init(cwd: string): Promise<void> {
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
  let customLibs: LibEntry[] = [];

  if (allDeps.length > 0) {
    const llmCandidates = await detectUnknownLibs(allDeps);

    // Decide which deps to show the user
    let candidates: string[];
    if (llmCandidates.length > 0) {
      candidates = llmCandidates;
    } else {
      // No LLM or LLM returned nothing — let user pick from all deps
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

    // Prompt for doc URL per candidate (LLM-filtered or user-selected)
    for (const name of candidates) {
      const input = await text({
        message: `Docs for "${name}"? (llms.txt URL, HTTP page, raw .md URL, or local path — Enter to skip)`,
        placeholder: "https://example.com/llms.txt",
      });
      if (isCancel(input) || !input?.trim()) continue;

      const trimmed = String(input).trim();
      customLibs.push({ name, ...inferSourceType(trimmed) });
    }
  }

  // 3. Prompt for Supabase URL and service key
  const supabaseUrl = await text({
    message: "Supabase URL (from supabase start or your hosted project):",
    placeholder: "http://localhost:54321",
    validate: v => (v.startsWith("http") ? undefined : "Must be a URL"),
  });
  if (isCancel(supabaseUrl)) { outro("Cancelled."); return; }

  const serviceKey = await text({
    message: "Supabase service role key:",
    validate: v => (v.length > 10 ? undefined : "Looks too short"),
  });
  if (isCancel(serviceKey)) { outro("Cancelled."); return; }

  // 3. Create Supabase client and upsert repo row
  const client = createClient(String(supabaseUrl), String(serviceKey), {
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

  // 4. Write .sensei/config.yaml and credentials
  const senseiDir = join(cwd, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  const customLibsYaml = customLibs.length > 0
    ? `custom_libs:\n${customLibs.map(l => {
        const urlField = l.base_url ? `    base_url: ${l.base_url}` : `    local_path: ${l.local_path}`;
        return `  - name: ${l.name}\n    source_type: ${l.source_type}\n${urlField}`;
      }).join("\n")}\n`
    : "";
  await writeFile(
    join(senseiDir, "config.yaml"),
    `repo_id: ${repoId}\nsupabase_url: ${String(supabaseUrl)}\n${customLibsYaml}`,
  );

  // Write credentials to ~/.config/sensei/ (global, not committed)
  const credsDir = join(homedir(), ".config", "sensei");
  await mkdir(credsDir, { recursive: true });
  const credsPath = join(credsDir, "credentials.yaml");
  await writeFile(credsPath, `supabase_service_key: ${String(serviceKey)}\n`);
  // Restrict credentials file to owner-only (service role key — never share)
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
    // Update last_indexed_at
    await client.from("repos").update({ last_indexed_at: new Date().toISOString() }).eq("id", repoId);
  } catch (err) {
    indexSpinner.stop(`Indexing failed: ${err instanceof Error ? err.message : String(err)}`);
    log.warn("You can re-index later with: sensei index");
  }

  // 6. Write CLAUDE.md and AGENTS.md
  await writeFile(join(cwd, "CLAUDE.md"), claudeMdTemplate({ repoName, stack, repoId }));
  await writeFile(join(cwd, "AGENTS.md"), agentsMdTemplate({ repoName, stack }));

  // 7. Install hooks
  const hookSpinner = spinner();
  hookSpinner.start("Installing collector hooks...");
  try {
    await installHooks({});
    hookSpinner.stop("Hooks installed");
  } catch (err) {
    hookSpinner.stop(`Hook install skipped: ${err instanceof Error ? err.message : String(err)}`);
  }

  note(
    [
      `Created: .sensei/config.yaml, CLAUDE.md, AGENTS.md`,
      ``,
      `Next steps:`,
      `  1. Start the dashboard: cd apps/dashboard && bun run dev`,
      `  2. Start the collector: sensei serve`,
      `  3. Add the MCP server to your agent config`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
