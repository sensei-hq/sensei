import { watchRepo } from "@sensei/graph-indexer";
import { loadSenseiConfig } from "@sensei/shared";

export interface WatchOptions {
  drift?: boolean;
}

export async function watch(repoPath: string, _options: WatchOptions = {}): Promise<void> {
  // Load config to get repoId
  const config = await loadSenseiConfig(repoPath);
  if (!config?.repo_id) {
    console.error("No .sensei/config.yaml found. Run sensei init first.");
    process.exit(1);
    return;
  }

  const repoId = config.repo_id;
  console.log(`Starting graph watcher for ${repoPath} (repo: ${repoId})...`);

  const handle = await watchRepo({
    repoPath,
    repoId,
    project: repoId,
    onUpdate: (result) => {
      const parts: string[] = [];
      if (result.added > 0) parts.push(`+${result.added} added`);
      if (result.updated > 0) parts.push(`~${result.updated} updated`);
      if (result.removed > 0) parts.push(`-${result.removed} removed`);
      if (parts.length > 0) {
        console.log(`[${new Date().toLocaleTimeString()}] Graph: ${parts.join(", ")} (${result.durationMs}ms)`);
      }
    },
  });

  console.log("Watching for changes... (Ctrl+C to stop)");

  await new Promise<void>((resolve) => {
    process.once("SIGINT", async () => {
      console.log("\nStopping watcher...");
      await handle.stop();
      resolve();
    });
  });
}
