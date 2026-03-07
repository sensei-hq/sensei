import { intro, outro, spinner, note, confirm, isCancel } from "@clack/prompts";
import { reindexRepo } from "../tools/reindex.js";
import { existsSync } from "fs";
import { join } from "path";

export async function add(cwd: string): Promise<void> {
  intro("sensei add");

  const alreadyIndexed = existsSync(join(cwd, ".sensei", "symbol-map.json"));

  if (alreadyIndexed) {
    const proceed = await confirm({
      message: ".sensei/ already exists. Re-index and update artifacts?",
    });
    if (isCancel(proceed) || !proceed) {
      outro("Cancelled.");
      return;
    }
  }

  const s = spinner();
  s.start("Indexing repo...");
  const summary = await reindexRepo(cwd);
  if (summary.forced) {
    const total = summary.added + summary.updated;
    s.stop(`Full scan: ${total} files indexed (${summary.added} new, ${summary.updated} updated)`);
  } else {
    s.stop(`${summary.updated} updated, ${summary.added} added, ${summary.removed} removed, ${summary.unchanged} unchanged`);
  }

  note(
    [
      `Edit .sensei/llmspec.yaml to declare doc coverage (docs[].covers[])`,
      `Run: sensei hooks install --drift   to enable pre-commit drift check`,
    ].join("\n"),
    "Next steps"
  );

  outro("Done.");
}
