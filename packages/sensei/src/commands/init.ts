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
      `Created: .llmspec.yaml, CLAUDE.md, llms.txt, .index/`,
      ``,
      `Next steps:`,
      `  1. Edit .llmspec.yaml to add doc coverage (docs[].covers[])`,
      `  2. Run: sensei hooks install --drift   to enable pre-commit drift check`,
      `  3. Commit .llmspec.yaml and .index/ to version control`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
