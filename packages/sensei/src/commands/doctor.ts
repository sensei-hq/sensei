import { intro, outro, spinner, note, confirm, isCancel, log } from "@clack/prompts";
import { readFile, writeFile, readdir } from "fs/promises";
import { join, extname, relative } from "path";
import { existsSync } from "fs";
import Anthropic from "@anthropic-ai/sdk";

const MODEL = "claude-opus-4-6";

async function callClaude(prompt: string): Promise<string> {
  const client = new Anthropic();
  const stream = client.messages.stream({
    model: MODEL,
    max_tokens: 8192,
    thinking: { type: "adaptive" },
    messages: [{ role: "user", content: prompt }],
  });
  const message = await stream.finalMessage();
  const text = message.content.find(b => b.type === "text");
  return text?.text ?? "";
}

const TEMPLATE_DIR_MAP: Array<{ pattern: RegExp; template: string }> = [
  { pattern: /^docs\/design\//,    template: "docs/templates/design.md" },
  { pattern: /^docs\/features\//,  template: "docs/templates/feature.md" },
  { pattern: /^docs\/requirements\//, template: "docs/templates/feature.md" },
];

function detectTemplate(filePath: string, repoPath: string): string | null {
  const rel = relative(repoPath, filePath).replace(/\\/g, "/");

  // Skip plans — they are implementation artifacts
  if (rel.startsWith("docs/plans/")) return null;

  for (const { pattern, template } of TEMPLATE_DIR_MAP) {
    if (pattern.test(rel)) return join(repoPath, template);
  }
  return null;
}

function buildPrompt(templateContent: string, existingContent: string): string {
  return `Reformat the following document to match the canonical template.

## Template

${templateContent}

## Existing Document

${existingContent}

## Rules

1. Preserve ALL existing information — restructure only, do not summarise away details
2. Add missing template sections with placeholder text: "TODO: [section description]"
3. Place any content that doesn't fit the template under "## Additional Notes"
4. Do not invent information — only reorganise what exists
5. Keep all code blocks, tables, and examples intact
6. Output the complete reformatted document only — no preamble or explanation`;
}

async function doctorFile(
  filePath: string,
  repoPath: string,
  options: { dryRun?: boolean; template?: string }
): Promise<boolean> {
  const templatePath = options.template ?? detectTemplate(filePath, repoPath);
  if (!templatePath) {
    log.warn(`Skipping ${relative(repoPath, filePath)} — no template detected`);
    return false;
  }
  if (!existsSync(templatePath)) {
    log.warn(`Template not found: ${templatePath}`);
    return false;
  }

  const [templateContent, existingContent] = await Promise.all([
    readFile(templatePath, "utf-8"),
    readFile(filePath, "utf-8"),
  ]);

  const prompt = buildPrompt(templateContent, existingContent);

  if (options.dryRun) {
    console.log("\n--- PROMPT ---\n");
    console.log(prompt);
    console.log("\n--- END PROMPT ---\n");
    return false;
  }

  const reformatted = await callClaude(prompt);
  if (!reformatted.trim()) {
    log.error(`Empty response from Claude for ${relative(repoPath, filePath)}`);
    return false;
  }

  await writeFile(filePath, reformatted, "utf-8");
  return true;
}

export async function doctor(
  targetPath: string,
  repoPath: string,
  options: { dryRun?: boolean; template?: string }
): Promise<void> {
  intro("sensei doctor");

  const fullPath = join(repoPath, targetPath);

  if (!existsSync(fullPath)) {
    log.error(`Path not found: ${fullPath}`);
    outro("Done.");
    return;
  }

  const { stat } = await import("fs/promises");
  const s = await stat(fullPath);

  if (!s.isDirectory()) {
    // Single file
    await doctorFile(fullPath, repoPath, options);
    outro("Done.");
    return;
  }

  // Directory batch
  const allFiles = await readdir(fullPath, { recursive: true }) as string[];
  const mdFiles = allFiles
    .filter(f => extname(f) === ".md")
    .map(f => join(fullPath, f));

  let done = 0;
  for (const file of mdFiles) {
    const rel = relative(repoPath, file);

    if (options.dryRun) {
      await doctorFile(file, repoPath, options);
      done++;
      continue;
    }

    const proceed = await confirm({ message: `Doctor ${rel}?` });
    if (isCancel(proceed)) { outro("Cancelled."); return; }
    if (!proceed) { log.info(`Skipped ${rel}`); continue; }

    const sp = spinner();
    sp.start(`Doctoring ${rel}...`);
    await doctorFile(file, repoPath, options);
    sp.stop(`Done: ${rel}`);
    done++;
  }

  note(`${done} file(s) processed`, "Summary");
  outro("Done.");
}
