import { readLlmSpec, readSymbolMap, readIndexFile } from "../index-reader.js";

export interface ContextSlice {
  scope: string;
  content: string;
  tokenEstimate: number;
}

export async function loadContext(repoPath: string, scope: string): Promise<ContextSlice> {
  let content = "";

  if (scope === "orientation") {
    const spec = await readLlmSpec(repoPath);
    content = `# ${spec.project}\n${spec.description}\n\nStack: ${spec.stack.join(", ")}\n\nEntry points:\n` +
      spec.entry_points.map(e => `- ${e.path}: ${e.role}`).join("\n");
  } else if (scope === "patterns") {
    content = await readIndexFile(repoPath, "patterns.md") ?? "No patterns indexed";
  } else if (scope === "stack") {
    content = await readIndexFile(repoPath, "stack.md") ?? "No stack info indexed";
  } else {
    const map = await readSymbolMap(repoPath);
    const relevant = Object.entries(map).filter(([f]) => f.startsWith(scope));
    if (relevant.length === 0) {
      content = `No exports found for scope '${scope}'.`;
    } else {
      content = relevant.map(([f, e]) => `### ${f}\n${e.L0.join("\n")}`).join("\n\n");
    }
  }

  return { scope, content, tokenEstimate: Math.ceil(content.length / 4) };
}

export async function recommendNext(repoPath: string, task: string): Promise<string> {
  const lower = task.toLowerCase();

  if (lower.match(/\b(list|find|search|what.*export|all.*function)\b/)) {
    return "Recommended: use list_exports() at L0. No full file loads needed.";
  }
  if (lower.match(/\b(fix|bug|error|failing|broken|debug)\b/)) {
    return "Recommended: load L2 for the relevant module, then L3 for the specific function. Use get_file_context(path, 'L2') first.";
  }
  if (lower.match(/\b(explain|understand|how|walk|trace|flow)\b/)) {
    return "Recommended: load orientation slice (load_context('orientation')), then L1 or L2 for relevant modules via get_file_context.";
  }
  if (lower.match(/\b(add|implement|create|build|write)\b/)) {
    return "Recommended: load patterns (load_context('patterns')), then L0 for related modules, then L3 only for the specific files you will edit.";
  }
  return "Recommended: start with get_llmspec() for orientation (~500 tokens), then load targeted slices as needed.";
}
