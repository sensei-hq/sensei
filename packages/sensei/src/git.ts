import { execSync } from "child_process";

function exec(cmd: string, cwd: string): string {
  return execSync(cmd, { cwd, encoding: "utf-8" }) as string;
}

export function getCurrentBranch(repoPath: string): string {
  return exec("git rev-parse --abbrev-ref HEAD", repoPath).trim();
}

export function isCleanWorkingTree(repoPath: string): boolean {
  return exec("git status --porcelain", repoPath).trim() === "";
}

export function branchExists(repoPath: string, branch: string): boolean {
  return exec(`git branch --list ${branch}`, repoPath).trim() !== "";
}

export function createAndCheckoutBranch(repoPath: string, branch: string, from: string): void {
  exec(`git checkout -b ${branch} ${from}`, repoPath);
}

export function checkoutBranch(repoPath: string, branch: string): void {
  exec(`git checkout ${branch}`, repoPath);
}

export function stageFiles(repoPath: string, paths: string[]): void {
  exec(`git add ${paths.map(p => `"${p}"`).join(" ")}`, repoPath);
}

export function commitFiles(repoPath: string, message: string): void {
  exec(`git commit -m ${JSON.stringify(message)}`, repoPath);
}

export function mergeBranch(repoPath: string, branch: string): void {
  exec(`git merge ${branch}`, repoPath);
}

export function deleteBranch(repoPath: string, branch: string): void {
  exec(`git branch -d ${branch}`, repoPath);
}

export function readFileFromBranch(repoPath: string, branch: string, filePath: string): string {
  return exec(`git show ${branch}:${filePath}`, repoPath);
}
