import { readFile, writeFile } from "fs/promises";
import { existsSync } from "fs";
import { execSync } from "child_process";
import yaml from "js-yaml";
import { generateCoverage } from "@sensei/tools";
import { senseiPath } from "@sensei/shared";

async function detectModel(): Promise<string | null> {
  try {
    const res = await fetch("http://127.0.0.1:11434/api/tags", { signal: AbortSignal.timeout(2000) });
    if (!res.ok) return null;
    const data = await res.json() as { models?: Array<{ name: string }> };
    const localModels = (data.models ?? [])
      .map(m => m.name)
      .filter(n => !n.includes(":cloud")); // skip remote/cloud models
    return localModels[0] ?? null;
  } catch {
    return null;
  }
}

interface LlmSpec {
  docs?: Array<{ path: string; covers?: string[] }>;
  [key: string]: unknown;
}

export async function benchmarkCoverage(repoPath: string): Promise<void> {
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  const expectedPath = senseiPath(repoPath, "llmspec-expected.yaml");

  if (!existsSync(llmspecPath)) {
    console.error("sensei: .sensei/llmspec.yaml not found. Run 'sensei init' first.");
    process.exit(1);
  }

  if (!existsSync(expectedPath)) {
    console.error("sensei: .sensei/llmspec-expected.yaml not found (gold standard required for scoring).");
    process.exit(1);
  }

  // Check Ollama is available and detect installed model
  const { OllamaBackend } = await import("@sensei/server");
  const probe = new OllamaBackend();
  if (!await probe.isAvailable()) {
    console.error("sensei: Ollama is not running. Start it with: ollama serve");
    process.exit(1);
  }

  // Pick the first available llama/qwen/mistral model
  const ollamaModel = await detectModel();
  if (!ollamaModel) {
    console.error("sensei: No local model found in Ollama. Pull one with: ollama pull llama3.2");
    process.exit(1);
  }
  console.log(`Using model: ${ollamaModel}`);
  const model = new OllamaBackend({ model: ollamaModel });

  console.log("Generating coverage mappings using local model...");
  console.log("(This may take a minute — each doc is analyzed by the model)");

  const entries = await generateCoverage(repoPath, model);

  if (entries.length === 0) {
    console.error("sensei: No docs found to analyze. Check that docs/ directory exists.");
    process.exit(1);
  }

  // Update llmspec.yaml docs[] in place
  const spec = yaml.load(await readFile(llmspecPath, "utf-8")) as LlmSpec;
  spec.docs = entries.map(e => ({ path: e.path, covers: e.covers }));
  await writeFile(llmspecPath, yaml.dump(spec, { lineWidth: 120 }));

  console.log(`\nUpdated .sensei/llmspec.yaml with ${entries.length} doc entries.`);

  // Score against gold standard
  const scoreScript = `${repoPath}/tasks/score-coverage.ts`;
  if (existsSync(scoreScript)) {
    try {
      const output = execSync(`bun ${scoreScript} ${llmspecPath}`, {
        cwd: repoPath,
        encoding: "utf-8",
      });
      console.log(output);
    } catch (err: unknown) {
      const e = err as { stdout?: string; message?: string };
      if (e.stdout) console.log(e.stdout);
      else console.error(e.message);
    }
  }
}
