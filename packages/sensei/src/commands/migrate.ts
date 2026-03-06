import { intro, outro, spinner, note, log } from "@clack/prompts";
import { readFile, mkdir, rename, readdir } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { addDecision, addPattern, checkpoint } from "../tools/project-memory.js";

export async function migrate(cwd: string): Promise<void> {
  intro("sensei migrate");

  const agentsDir = join(cwd, "agents");
  if (!existsSync(agentsDir)) {
    log.warn("No agents/ folder found. Nothing to migrate.");
    outro("Done.");
    return;
  }

  await mkdir(join(cwd, ".index/checkpoints/sessions"), { recursive: true });

  const s = spinner();
  s.start("Reading agents/ files...");

  const migrated: string[] = [];

  // Migrate memory.md → decisions in memory.yaml
  const memoryPath = join(agentsDir, "memory.md");
  if (existsSync(memoryPath)) {
    const content = await readFile(memoryPath, "utf-8");
    const decisions = content.split("\n")
      .filter(l => l.match(/^[-*]\s+/))
      .map(l => l.replace(/^[-*]\s+/, "").trim())
      .filter(l => l.length > 0);
    for (const decision of decisions) {
      await addDecision(cwd, decision);
    }
    migrated.push(`memory.md → ${decisions.length} decisions`);
  }

  // Migrate design-patterns.md → patterns.yaml
  const patternsPath = join(agentsDir, "design-patterns.md");
  if (existsSync(patternsPath)) {
    const content = await readFile(patternsPath, "utf-8");
    const sections = content.split(/\n## |\n### /).slice(1);
    for (const section of sections) {
      const [name, ...rest] = section.split("\n");
      const convention = rest
        .filter(l => l.match(/^[-*]\s+/))
        .map(l => l.replace(/^[-*]\s+/, ""))
        .join("; ");
      if (name && convention) await addPattern(cwd, name.trim(), convention.trim());
    }
    migrated.push(`design-patterns.md → ${sections.length} patterns`);
  }

  // Capture last journal entries as session checkpoint
  const journalPath = join(agentsDir, "journal.md");
  if (existsSync(journalPath)) {
    const content = await readFile(journalPath, "utf-8");
    const lastEntries = content.trim().split("\n").slice(-10).join("\n");
    await checkpoint(cwd, `Migrated from agents/journal.md. Last entries: ${lastEntries.slice(0, 200)}`);
    migrated.push("journal.md → session checkpoint");
  }

  s.stop("Migration complete.");

  // Archive agents/ folder
  s.start("Archiving agents/ folder...");
  const archiveDir = join(cwd, "agents/_archived");
  await mkdir(archiveDir, { recursive: true });
  const files = await readdir(agentsDir);
  for (const file of files) {
    if (file === "_archived") continue;
    await rename(join(agentsDir, file), join(archiveDir, file));
  }
  s.stop("Archived to agents/_archived/");

  note(
    [
      ...migrated,
      "",
      "agents/ archived to agents/_archived/",
      "Review .index/checkpoints/ to verify parity",
      "Delete agents/_archived/ when satisfied",
      "",
      "Resume with: get_session_context()",
    ].join("\n"),
    "Migration summary"
  );

  outro("Done. Run sensei status to verify.");
}
