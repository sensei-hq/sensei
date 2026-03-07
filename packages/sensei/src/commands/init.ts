import { intro, outro, spinner, note, confirm, log, isCancel } from "@clack/prompts";
import { reindexRepo } from "../tools/reindex.js";
import { checkSystemRequirements, OLLAMA_MODEL, OLLAMA_MODEL_SIZE_GB, OLLAMA_BASE_URL } from "../model/system-check.js";
import type { SetupStatus } from "@sensei/shared";

export async function init(cwd: string): Promise<void> {
  intro("sensei init");

  // --- Prerequisites check ---
  const checkSpinner = spinner();
  checkSpinner.start("Checking prerequisites...");
  let status: SetupStatus;
  try {
    status = await checkSystemRequirements();
    checkSpinner.stop("Prerequisites checked");
  } catch (err) {
    checkSpinner.stop("Prerequisites check failed");
    log.warn(`Could not check prerequisites: ${err instanceof Error ? err.message : String(err)}`);
    status = {
      ollamaBinary: false, ollamaRunning: false, ollamaModel: false,
      ollamaModelName: "", onnxModel: false, diskFreeGB: 0,
      ramTotalGB: 0, ramAvailableGB: 0,
    };
  }

  const needs: string[] = [];
  if (!status.ollamaBinary) needs.push(`Ollama not installed  (needed for local model inference)`);
  if (status.ollamaBinary && !status.ollamaRunning) needs.push(`Ollama not running   (start: ollama serve)`);
  if (status.ollamaBinary && status.ollamaRunning && !status.ollamaModel)
    needs.push(`Model not pulled     ${OLLAMA_MODEL} (~${OLLAMA_MODEL_SIZE_GB} GB, 4 GB RAM)`);
  if (!status.onnxModel) needs.push(`Embedding model      Xenova/all-MiniLM-L6-v2 (22 MB, auto-download)`);

  if (needs.length > 0) {
    note(
      [
        "Some components are not set up. Sensei will use regex indexing until they are available.",
        "",
        "Missing:",
        ...needs.map(n => `  ✗ ${n}`),
        "",
        `Disk free: ${status.diskFreeGB.toFixed(1)} GB   RAM available: ${status.ramAvailableGB.toFixed(1)} GB`,
        "",
        `Set up later with: sensei serve  (then: sensei server status)`,
      ].join("\n"),
      "Setup status"
    );

    if (status.ollamaBinary && status.ollamaRunning && !status.ollamaModel) {
      if (status.diskFreeGB < OLLAMA_MODEL_SIZE_GB + 0.5) {
        log.warn(`Not enough disk space to pull ${OLLAMA_MODEL} (need ${OLLAMA_MODEL_SIZE_GB + 0.5} GB, have ${status.diskFreeGB.toFixed(1)} GB)`);
      } else {
        const pull = await confirm({ message: `Pull ${OLLAMA_MODEL} now? (~${OLLAMA_MODEL_SIZE_GB} GB)` });
        if (!isCancel(pull) && pull) {
          const pullSpinner = spinner();
          pullSpinner.start(`Pulling ${OLLAMA_MODEL}...`);
          try {
            const res = await fetch(`${OLLAMA_BASE_URL}/api/pull`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ name: OLLAMA_MODEL, stream: false }),
              signal: AbortSignal.timeout(600_000),
            });
            if (res.ok) {
              pullSpinner.stop(`${OLLAMA_MODEL} pulled`);
            } else {
              pullSpinner.stop(`Pull failed (status ${res.status})`);
            }
          } catch (err) {
            pullSpinner.stop(`Pull failed: ${err instanceof Error ? err.message : String(err)}`);
          }
        }
      }
    }
  }

  // --- Index ---
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
      `  4. Run: sensei serve   to start inference server for richer analysis`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
