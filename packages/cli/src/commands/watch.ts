// packages/cli/src/commands/watch.ts
import chokidar from "chokidar";
import { join } from "path";
import { existsSync } from "fs";
import { reindexRepo } from "@sensei/tools";

export async function watch(repoPath: string): Promise<void> {
  const watchTargets = [
    join(repoPath, "src"),
    join(repoPath, "docs"),
    join(repoPath, "package.json"),
  ].filter(p => existsSync(p));

  if (watchTargets.length === 0) {
    console.log("Nothing to watch — no src/, docs/, or package.json found.");
    return;
  }

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let reindexPromise: Promise<void> | null = null;

  const watcher = chokidar.watch(watchTargets, {
    ignored: [
      /\.sensei\//,
      /node_modules/,
      /\.git\//,
    ],
    ignoreInitial: true,
    persistent: true,
  });

  function triggerReindex() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      debounceTimer = null;
      if (reindexPromise) return; // skip — reindex in flight
      const start = Date.now();
      reindexPromise = reindexRepo(repoPath)
        .then(summary => {
          const changed = summary.added + summary.updated + summary.removed;
          const elapsed = Date.now() - start;
          console.log(`reindexed ${changed} files (${elapsed}ms)`);
        })
        .catch(err => console.error("reindex error:", err.message))
        .finally(() => { reindexPromise = null; });
      // No await here — setTimeout discards the callback's return value
    }, 500);
  }

  watcher.on("change", triggerReindex);
  watcher.on("add", triggerReindex);
  watcher.on("unlink", triggerReindex);

  console.log(`Watching ${watchTargets.map(p => p.replace(repoPath + "/", "")).join(", ")}... (Ctrl+C to stop)`);

  await new Promise<void>(resolve => {
    process.once("SIGINT", async () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      const inFlight = reindexPromise;
      if (inFlight) await inFlight;
      await watcher.close();
      console.log("Watch stopped.");
      resolve();
    });
  });
}
