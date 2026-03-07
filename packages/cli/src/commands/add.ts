import { intro, outro, spinner, note, confirm, isCancel } from "@clack/prompts";
import { reindexRepo } from "@sensei/tools";
import { existsSync } from "fs";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";

export async function add(cwd: string): Promise<void> {
  intro("sensei add");

  const alreadyIndexed = existsSync(senseiPath(cwd, "symbol-map.json"));

  if (alreadyIndexed) {
    const proceed = await confirm({
      message: `${SENSEI_DIR}/ already exists. Re-index and update artifacts?`,
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
      `Edit ${SENSEI_DIR}/llmspec.yaml to declare doc coverage (docs[].covers[])`,
      `Run: sensei hooks install --drift   to enable pre-commit drift check`,
    ].join("\n"),
    "Next steps"
  );

  outro("Done.");
}
