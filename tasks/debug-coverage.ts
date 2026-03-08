import { OllamaBackend } from "../packages/server/src/index.ts";
import { readFileSync } from "fs";

const model = new OllamaBackend();
const symbolMap = JSON.parse(readFileSync(".sensei/symbol-map.json", "utf8"));

const sourceFiles = Object.entries(symbolMap as Record<string, { L0: string[] }>)
  .filter(([path]) => !path.endsWith(".md") && !path.includes(".spec."))
  .map(([path, entry]) => ({ path, exports: entry.L0.slice(0, 5).join(", ") || "(no exports)" }));

console.log("Source files count:", sourceFiles.length);
console.log("Sample:", sourceFiles.slice(0, 3).map(f => f.path));

const docContent = readFileSync("docs/design/09-cli.md", "utf8").slice(0, 1500);

const prompt = `You are analyzing a documentation file to determine which source code files it documents.

Documentation file: docs/design/09-cli.md

Documentation content:
---
${docContent}
---

Available source files (path: key exports):
${sourceFiles.map(f => `- ${f.path}: ${f.exports}`).join("\n")}

Which of these source files does this documentation primarily describe or cover?
Respond with ONLY a JSON array of file paths from the list above.
Example: ["packages/tools/src/tools/reindex.ts"]
If none match, respond with [].`;

console.log("\nPrompt length:", prompt.length);
console.log("\n--- Calling Ollama ---");
const response = await model.generate(prompt);
console.log("\n=== MODEL RESPONSE ===");
console.log(response);
