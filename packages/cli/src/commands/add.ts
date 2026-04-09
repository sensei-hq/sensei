import { intro, outro, spinner, note, confirm, isCancel } from "@clack/prompts";
import { indexRepo } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";
import { existsSync } from "fs";
import { join } from "path";
import { homedir } from "os";

export async function add(cwd: string): Promise<void> {
  intro("sensei add");

  const config = await loadSenseiConfig(cwd);
  if (!config?.repo_id) {
    outro("No .sensei/config.yaml found. Run sensei init first.");
    return;
  }

  const repoId = config.repo_id;
  const graphPath = join(homedir(), ".sensei", "projects", repoId, "graph.kuzu");
  const alreadyIndexed = existsSync(graphPath);

  if (alreadyIndexed) {
    const proceed = await confirm({
      message: `Graph already exists for this repo. Re-index now?`,
    });
    if (isCancel(proceed) || !proceed) {
      outro("Cancelled.");
      return;
    }
  }

  const s = spinner();
  s.start("Indexing repo into graph...");
  const result = await indexRepo({ repoPath: cwd, repoId, project: repoId });
  s.stop(`Indexed ${result.filesIndexed} files — ${result.functionsIndexed} functions, ${result.typesIndexed} types (${result.durationMs}ms)`);

  note(
    [
      `Run: sensei watch   to keep the graph up-to-date`,
      `Run: sensei serve   to start the MCP server`,
    ].join("\n"),
    "Next steps"
  );

  outro("Done.");
}
