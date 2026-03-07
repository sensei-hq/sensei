import { writeFile } from "fs/promises";
import { join } from "path";
import { intro, outro, select, text, isCancel, note, log, confirm } from "@clack/prompts";
import {
  branchExists, checkoutBranch, mergeBranch, deleteBranch, readFileFromBranch,
} from "../git.js";

// ── Pure helpers (exported for testing) ────────────────────────────────────────

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

export async function benchmarkPromote(runName: string, repoPath: string): Promise<void> {
  intro("sensei benchmark promote");

  const branches = {
    a: `benchmark/${runName}-a`,
    b: `benchmark/${runName}-b`,
    c: `benchmark/${runName}-c`,
  };

  // ── Verify branches exist ────────────────────────────────────────────────────
  for (const [, branch] of Object.entries(branches)) {
    if (!branchExists(repoPath, branch)) {
      log.error(`Branch not found: ${branch}`);
      outro("Aborted.");
      return;
    }
  }

  // ── Read results JSON from branch a ─────────────────────────────────────────
  const resultsFile = `.sensei/benchmark-${runName}.json`;
  let data: any;
  try {
    data = JSON.parse(readFileFromBranch(repoPath, branches.a, resultsFile));
  } catch {
    log.error(`Could not read ${resultsFile} from ${branches.a}`);
    outro("Aborted.");
    return;
  }

  const { autoPromoted, baseBranch, report } = data;
  const results = report?.results ?? data.scores;
  const strategyNames: Record<string, string> = {
    a: report?.strategies?.a?.name ?? "Targeted index",
    b: report?.strategies?.b?.name ?? "Raw content",
    c: report?.strategies?.c?.name ?? "Full repo index",
  };

  // ── Display comparison table ─────────────────────────────────────────────────
  note(
    (["a", "b", "c"] as const).map(k => {
      const r = results[k];
      const marker = k === autoPromoted ? " ← auto-winner" : "";
      return `${k.toUpperCase()} (${strategyNames[k]}): struct=${r.structuralScore} judge=${r.judgeScore} tokens=${r.tokensIn}→${r.tokensOut} files=${r.filesGenerated}${marker}`;
    }).join("\n"),
    "Benchmark results"
  );

  // ── Pick strategy ────────────────────────────────────────────────────────────
  const preferred = await select({
    message: "Which strategy to promote?",
    options: [
      { value: "a", label: `A — ${strategyNames.a} (struct=${results.a.structuralScore} judge=${results.a.judgeScore})` },
      { value: "b", label: `B — ${strategyNames.b} (struct=${results.b.structuralScore} judge=${results.b.judgeScore})` },
      { value: "c", label: `C — ${strategyNames.c} (struct=${results.c.structuralScore} judge=${results.c.judgeScore})` },
    ],
  });
  if (isCancel(preferred)) { outro("Cancelled."); return; }

  const noteText = await text({ message: "Optional note (press Enter to skip):", placeholder: "" });
  if (isCancel(noteText)) { outro("Cancelled."); return; }

  // ── Git permission prompt ────────────────────────────────────────────────────
  const chosenBranch = branches[preferred as "a" | "b" | "c"];
  log.info(
    `sensei will perform these git operations:\n` +
    `  git checkout ${baseBranch}\n` +
    `  git merge ${chosenBranch}`
  );
  const okMerge = await confirm({ message: "Proceed with merge?" });
  if (isCancel(okMerge) || !okMerge) { outro("Cancelled."); return; }

  // ── Merge ────────────────────────────────────────────────────────────────────
  checkoutBranch(repoPath, baseBranch);
  mergeBranch(repoPath, chosenBranch);

  // ── Update results JSON and commit ───────────────────────────────────────────
  const feedback = buildFeedback(preferred as string, autoPromoted, (noteText as string) || undefined);
  data.userFeedback = feedback;
  data.promoted = preferred;
  if (data.report) {
    data.report.userFeedback = feedback;
    data.report.promoted = preferred;
  }
  await writeFile(join(repoPath, resultsFile), JSON.stringify(data, null, 2), "utf-8");

  // ── Submit telemetry ─────────────────────────────────────────────────────────
  const telemetryUrl = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
  submitReport(data.report ?? data, telemetryUrl).catch(() => {});

  // ── Offer to delete other branches ───────────────────────────────────────────
  const losers = (["a", "b", "c"] as const)
    .filter(k => k !== preferred)
    .map(k => branches[k]);
  log.info(`Other benchmark branches:\n${losers.map(b => `  ${b}`).join("\n")}`);
  const okDelete = await confirm({ message: `Delete ${losers.join(" and ")}?` });
  if (!isCancel(okDelete) && okDelete) {
    for (const branch of losers) {
      deleteBranch(repoPath, branch);
      log.success(`Deleted ${branch}`);
    }
  }

  note(`Promoted: Strategy ${(preferred as string).toUpperCase()} (${strategyNames[preferred as string]})`, "Done");
  outro("Done.");
}
