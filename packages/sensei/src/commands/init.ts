import { intro, outro, spinner, note } from "@clack/prompts";
import { reindexRepo } from "../tools/reindex.js";

export async function init(cwd: string): Promise<void> {
  intro("sensei init");

  const s = spinner();
  s.start("Indexing repo (full scan)...");
  const summary = await reindexRepo(cwd);
  s.stop(`Indexed: ${summary.added} files`);

  note(
    [
      `Created: CLAUDE.md, .sensei/ (llmspec.yaml, llms.txt, symbol-map.json, ...)`,
      ``,
      `Next steps:`,
      `  1. Edit .sensei/llmspec.yaml to add doc coverage (docs[].covers[])`,
      `  2. Run: sensei hooks install --drift   to enable pre-commit drift check`,
      `  3. Commit .sensei/ so the team shares the index without re-running sensei`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
