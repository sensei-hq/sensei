import { readFile, writeFile, mkdir } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname, relative } from "path";
import { intro, outro, select, text, isCancel, note, log } from "@clack/prompts";

// ── Pure helpers (exported for testing) ────────────────────────────────────────

export function pickPreferred(scores: Record<string, { structuralScore: number; judgeScore: number }>): string {
  return Object.entries(scores)
    .map(([k, v]) => ({ k, score: v.structuralScore + v.judgeScore }))
    .reduce((best, cur) => cur.score > best.score ? cur : best).k;
}

export function buildFeedback(
  preferred: string,
  autoPromoted: string,
  noteText?: string,
): { preferred: string; systemAgreed: boolean; note?: string } {
  const fb: { preferred: string; systemAgreed: boolean; note?: string } = {
    preferred,
    systemAgreed: preferred === autoPromoted,
  };
  if (noteText) fb.note = noteText;
  return fb;
}

export async function submitReport(report: unknown, baseUrl: string): Promise<void> {
  try {
    await fetch(`${baseUrl}/reports`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report),
    });
  } catch {
    // Telemetry is best-effort — never throw
  }
}

// ── Main command ────────────────────────────────────────────────────────────────

export async function benchmarkPromote(resultsDir: string, repoPath: string): Promise<void> {
  intro("sensei benchmark promote");

  const resultsPath = join(repoPath, resultsDir, "results.json");
  if (!existsSync(resultsPath)) {
    log.error(`results.json not found: ${resultsPath}`);
    outro("Aborted.");
    return;
  }

  const data = JSON.parse(await readFile(resultsPath, "utf-8"));
  const { scores, autoPromoted, outputName, input } = data;

  // Display comparison table
  note(
    Object.entries(scores as Record<string, { structuralScore: number; judgeScore: number; tokensIn: number; filesGenerated: number }>)
      .map(([k, v]) => {
        const marker = k === autoPromoted ? " ← auto" : "";
        return `Strategy ${k.toUpperCase()}: struct=${v.structuralScore} judge=${(scores[k] as { judgeScore: number }).judgeScore} tokens=${v.tokensIn} files=${v.filesGenerated}${marker}`;
      })
      .join("\n"),
    "Benchmark results"
  );

  const preferred = await select({
    message: "Which strategy do you prefer?",
    options: [
      { value: "a", label: `A — Targeted index (struct=${scores.a.structuralScore} judge=${scores.a.judgeScore})` },
      { value: "b", label: `B — Raw content    (struct=${scores.b.structuralScore} judge=${scores.b.judgeScore})` },
      { value: "c", label: `C — Full repo index(struct=${scores.c.structuralScore} judge=${scores.c.judgeScore})` },
    ],
  });
  if (isCancel(preferred)) { outro("Cancelled."); return; }

  const noteText = await text({ message: "Optional note (press Enter to skip):", placeholder: "" });
  if (isCancel(noteText)) { outro("Cancelled."); return; }

  // Copy files if user's choice differs from auto-promoted
  if (preferred !== autoPromoted) {
    const srcDir = join(repoPath, resultsDir, preferred as string, outputName);
    const relInput = relative(repoPath, join(repoPath, input));
    const targetDir = join(repoPath, dirname(relInput), outputName);
    await mkdir(targetDir, { recursive: true });

    const { readdirSync, readFileSync } = await import("fs");
    for (const f of readdirSync(srcDir)) {
      const content = readFileSync(join(srcDir, f), "utf-8");
      await writeFile(join(targetDir, f), content, "utf-8");
    }
    log.success(`Copied Strategy ${(preferred as string).toUpperCase()} → ${relative(repoPath, targetDir)}/`);
  }

  // Update results.json
  const feedback = buildFeedback(preferred as string, autoPromoted, noteText as string || undefined);
  data.userFeedback = feedback;
  data.promoted = preferred;
  if (data.report) {
    data.report.userFeedback = feedback;
    data.report.promoted = preferred;
  }
  await writeFile(resultsPath, JSON.stringify(data, null, 2), "utf-8");

  // Submit telemetry (fire-and-forget)
  const telemetryUrl = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
  submitReport(data.report ?? data, telemetryUrl).catch(() => {});

  note(`Promoted: Strategy ${(preferred as string).toUpperCase()}`, "Done");
  outro("Done.");
}
