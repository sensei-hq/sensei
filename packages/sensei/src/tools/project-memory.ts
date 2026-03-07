import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import yaml from "js-yaml";
import { SENSEI_DIR } from "../constants.js";

const CHECKPOINTS = `${SENSEI_DIR}/checkpoints`;

interface Memory {
  version: number;
  decisions: Array<{ id: string; text: string; date: string }>;
  context: Record<string, string>;
}

interface Patterns {
  version: number;
  patterns: Array<{ name: string; convention: string; uses: number; added: string }>;
}

interface OpenItems {
  version: number;
  items: Array<{ id: string; question: string; added: string; status: string; resolution?: string }>;
}

async function readYaml<T>(path: string): Promise<T | null> {
  try {
    return yaml.load(await readFile(path, "utf-8")) as T;
  } catch { return null; }
}

async function writeYaml(path: string, data: unknown): Promise<void> {
  await writeFile(path, yaml.dump(data));
}

function today(): string {
  return new Date().toISOString().split("T")[0];
}

export async function getSessionContext(repoPath: string): Promise<string> {
  const dir = join(repoPath, CHECKPOINTS);
  if (!existsSync(dir)) {
    return "No session context found. Run sensei init or reindex_repo first.";
  }

  const [memory, items, patterns] = await Promise.all([
    readYaml<Memory>(join(dir, "memory.yaml")),
    readYaml<OpenItems>(join(dir, "open-items.yaml")),
    readYaml<Patterns>(join(dir, "patterns.yaml")),
  ]);

  const lines: string[] = ["## Project Memory"];

  if (memory?.context && Object.keys(memory.context).length) {
    Object.entries(memory.context).forEach(([k, v]) => lines.push(`${k}: ${v}`));
  }

  if (memory?.decisions?.length) {
    lines.push("\nDecisions:");
    memory.decisions.forEach(d => lines.push(`- ${d.text}`));
  }

  if (patterns?.patterns?.length) {
    lines.push("\nPatterns:");
    patterns.patterns.forEach(p => lines.push(`- ${p.name}: ${p.convention}`));
  }

  const open = items?.items.filter(i => i.status === "open") ?? [];
  if (open.length) {
    lines.push("\n## Open Items");
    open.forEach(i => lines.push(`- [${i.id}] ${i.question}`));
  }

  return lines.join("\n");
}

export async function addDecision(repoPath: string, text: string): Promise<string> {
  const path = join(repoPath, CHECKPOINTS, "memory.yaml");
  const memory = await readYaml<Memory>(path) ?? { version: 1, decisions: [], context: {} };
  const exists = memory.decisions.some(d => d.text.toLowerCase() === text.toLowerCase());
  if (!exists) {
    const id = text.toLowerCase().replace(/\W+/g, "-").slice(0, 40);
    memory.decisions.push({ id, text, date: today() });
    await writeYaml(path, memory);
  }
  return "Decision recorded.";
}

export async function addPattern(repoPath: string, name: string, convention: string): Promise<string> {
  const path = join(repoPath, CHECKPOINTS, "patterns.yaml");
  const patterns = await readYaml<Patterns>(path) ?? { version: 1, patterns: [] };
  const existing = patterns.patterns.find(p => p.name === name);
  if (existing) {
    existing.uses++;
  } else {
    patterns.patterns.push({ name, convention, uses: 1, added: today() });
  }
  await writeYaml(path, patterns);
  return "Pattern recorded.";
}

export async function askQuestion(repoPath: string, question: string): Promise<string> {
  const path = join(repoPath, CHECKPOINTS, "open-items.yaml");
  const items = await readYaml<OpenItems>(path) ?? { version: 1, items: [] };
  const id = `q-${Date.now()}`;
  items.items.push({ id, question, added: today(), status: "open" });
  await writeYaml(path, items);
  return id;
}

export async function getOpenItems(repoPath: string): Promise<string> {
  const path = join(repoPath, CHECKPOINTS, "open-items.yaml");
  const items = await readYaml<OpenItems>(path);
  const open = items?.items.filter(i => i.status === "open") ?? [];
  if (!open.length) return "No open items.";
  return open.map(i => `- [${i.id}] ${i.question}`).join("\n");
}

export async function closeItem(repoPath: string, id: string, resolution?: string): Promise<string> {
  const path = join(repoPath, CHECKPOINTS, "open-items.yaml");
  const items = await readYaml<OpenItems>(path);
  if (!items) return "No open items found.";
  const item = items.items.find(i => i.id === id);
  if (!item) return `Item '${id}' not found.`;
  item.status = "resolved";
  if (resolution) item.resolution = resolution;
  await writeYaml(path, items);
  return "Item closed.";
}

export async function checkpoint(
  repoPath: string,
  summary: string,
  decisions?: string[],
  patterns?: string[],
): Promise<string> {
  const dir = join(repoPath, CHECKPOINTS);
  await mkdir(join(dir, "sessions"), { recursive: true });

  if (decisions?.length) {
    await Promise.all(decisions.map(d => addDecision(repoPath, d)));
  }
  if (patterns?.length) {
    await Promise.all(patterns.map(p => addPattern(repoPath, p, p)));
  }

  const sessionFile = join(dir, "sessions", `${today()}-${Date.now()}.yaml`);
  await writeYaml(sessionFile, {
    date: today(),
    summary,
    decisions_added: decisions ?? [],
    patterns_added: patterns ?? [],
    timestamp: new Date().toISOString(),
  });

  return `Checkpointed. Resume with get_session_context().`;
}
