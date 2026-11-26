// Shared watched-root fixtures for watcher/index e2e — one helper instead of
// the per-spec mkdir+git-init copies. Everything lands under caller-chosen
// dirs (specs use /tmp/sensei-e2e-*) and cleans up after itself.
import { execFileSync } from 'child_process';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'fs';
import { dirname, join } from 'path';

// GIT_* from the caller (a hook in a linked worktree exports GIT_DIR and
// GIT_WORK_TREE) would aim every command at the caller's repo, not `dir`.
const isolatedEnv = (): NodeJS.ProcessEnv =>
  Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_')));

const git = (dir: string, args: string[]) =>
  execFileSync('git', args, { cwd: dir, stdio: 'ignore', env: isolatedEnv() });

/** A committed git repo at `dir` with `files` written into it. Idempotent:
// a second call leaves an existing repo (and its history) intact. */
export function makeGitRepo(dir: string, files: Record<string, string> = {}): string {
  mkdirSync(dir, { recursive: true });
  for (const [rel, content] of Object.entries(files)) {
    writeRepoFile(dir, rel, content);
  }
  if (existsSync(join(dir, '.git'))) return dir;
  git(dir, ['init']);
  git(dir, ['config', 'user.email', 'test@sensei.test']);
  git(dir, ['config', 'user.name', 'Sensei Test']);
  git(dir, ['add', '.']);
  git(dir, ['commit', '-m', 'init', '--allow-empty']);
  return dir;
}

/** Write (or overwrite) one file inside a repo, creating parents. */
export function writeRepoFile(dir: string, rel: string, content: string): string {
  const abs = join(dir, rel);
  mkdirSync(dirname(abs), { recursive: true });
  writeFileSync(abs, content);
  return abs;
}

/** Remove one file inside a repo; missing files are fine. */
export function removeRepoFile(dir: string, rel: string): void {
  rmSync(join(dir, rel), { force: true });
}

/** Remove a fixture tree; missing trees are fine. */
export function cleanupCorpus(dir: string): void {
  rmSync(dir, { recursive: true, force: true });
}
