import { intro, outro, spinner, note, log, isCancel, text } from "@clack/prompts";
import { writeFile, mkdir, access, readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { createClient } from "@supabase/supabase-js";
import { indexRepo } from "@sensei/engine";
import { claudeMdTemplate } from "../templates/claude-md.js";
import { agentsMdTemplate } from "../templates/agents-md.js";
import { installHooks } from "@sensei/collector";

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

  // 2. Prompt for Supabase URL and service key
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
  await writeFile(join(senseiDir, "config.yaml"), `repo_id: ${repoId}\nsupabase_url: ${String(supabaseUrl)}\n`);

  // Write credentials to ~/.config/sensei/ (global, not committed)
  const credsDir = join(homedir(), ".config", "sensei");
  await mkdir(credsDir, { recursive: true });
  const credsPath = join(credsDir, "credentials.yaml");
  await writeFile(credsPath, `supabase_service_key: ${String(serviceKey)}\n`);
  // Restrict credentials file to owner-only (service role key — never share)
  const { chmod } = await import("fs/promises");
  await chmod(credsPath, 0o600);

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
