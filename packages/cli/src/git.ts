import { execSync } from "child_process";
import { existsSync } from "fs";
import { join, dirname } from "path";

function exec(cmd: string, cwd: string): string {
  return execSync(cmd, { cwd, encoding: "utf-8" });
}

/**
 * Find the repo root starting from `cwd`.
 * Tries `git rev-parse --show-toplevel`, then walks up looking for package.json.
 * Returns the root path, or `cwd` if nothing is found.
 */
export function findRepoRoot(cwd: string): string {
  try {
    return exec("git rev-parse --show-toplevel", cwd).trim();
  } catch {
    // Not a git repo — walk up to find package.json
    let dir = cwd;
    while (true) {
      if (existsSync(join(dir, "package.json"))) return dir;
      const parent = dirname(dir);
      if (parent === dir) return cwd; // filesystem root
      dir = parent;
    }
  }
}

export function getCurrentBranch(repoPath: string): string {
  return exec("git rev-parse --abbrev-ref HEAD", repoPath).trim();
}

export function isCleanWorkingTree(repoPath: string): boolean {
  return exec("git status --porcelain", repoPath).trim() === "";
}

export function branchExists(repoPath: string, branch: string): boolean {
  return exec(`git branch --list ${JSON.stringify(branch)}`, repoPath).trim() !== "";
}

export function createAndCheckoutBranch(repoPath: string, branch: string, from: string): void {
  exec(`git checkout -b ${JSON.stringify(branch)} ${JSON.stringify(from)}`, repoPath);
}

export function checkoutBranch(repoPath: string, branch: string): void {
  exec(`git checkout ${JSON.stringify(branch)}`, repoPath);
}

export function stageFiles(repoPath: string, paths: string[]): void {
  exec(`git add ${paths.map(p => JSON.stringify(p)).join(" ")}`, repoPath);
}

export function commitFiles(repoPath: string, message: string): void {
  exec(`git commit -m ${JSON.stringify(message)}`, repoPath);
}

export function mergeBranch(repoPath: string, branch: string): void {
  exec(`git merge ${JSON.stringify(branch)}`, repoPath);
}

export function deleteBranch(repoPath: string, branch: string): void {
  exec(`git branch -d ${JSON.stringify(branch)}`, repoPath);
}

export function readFileFromBranch(repoPath: string, branch: string, filePath: string): string {
  return exec(`git show ${JSON.stringify(`${branch}:${filePath}`)}`, repoPath);
}
