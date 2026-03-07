import { intro, outro, log, confirm, isCancel } from "@clack/prompts";
import { branchExists, checkoutBranch } from "../git.js";

export function resolveBranchName(runBranch: string): string {
  return runBranch.startsWith("benchmark/") ? runBranch : `benchmark/${runBranch}`;
}

export async function benchmarkInspect(runBranch: string, repoPath: string): Promise<void> {
  intro("sensei benchmark inspect");

  const branch = resolveBranchName(runBranch);

  if (!branchExists(repoPath, branch)) {
    log.error(`Branch not found: ${branch}`);
    outro("Aborted.");
    return;
  }

  log.info(`sensei will perform:\n  git checkout ${branch}`);
  const ok = await confirm({ message: "Proceed?" });
  if (isCancel(ok) || !ok) { outro("Cancelled."); return; }

  checkoutBranch(repoPath, branch);
  log.success(`Switched to ${branch}`);
  outro("Done.");
}
