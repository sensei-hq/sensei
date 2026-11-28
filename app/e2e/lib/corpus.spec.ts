import { describe, it, expect, afterEach } from 'vitest';
import { execFileSync } from 'child_process';
import { mkdtempSync, existsSync, readFileSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { makeGitRepo, writeRepoFile, removeRepoFile, cleanupCorpus } from './corpus.js';

// Shared watched-root fixtures for watcher/index e2e: one helper instead of
// the per-spec mkdir+git-init copies. Everything lands under the OS temp dir
// and cleans up after itself.
describe('corpus fixtures', () => {
  const dirs: string[] = [];
  const sandbox = () => {
    const d = mkdtempSync(join(tmpdir(), 'sensei-corpus-spec-'));
    dirs.push(d);
    return d;
  };
  afterEach(() => {
    for (const d of dirs.splice(0)) cleanupCorpus(d);
  });

  it('makes a committed git repo with the given files', () => {
    const dir = join(sandbox(), 'repo');
    makeGitRepo(dir, { 'src/index.ts': 'export const x = 1;\n' });
    expect(existsSync(join(dir, '.git'))).toBe(true);
    expect(readFileSync(join(dir, 'src/index.ts'), 'utf8')).toBe('export const x = 1;\n');
  });

  it('is idempotent: a second call leaves the repo intact', () => {
    const dir = join(sandbox(), 'repo');
    makeGitRepo(dir);
    makeGitRepo(dir);
    expect(existsSync(join(dir, '.git'))).toBe(true);
  });

  it('writes, overwrites, and removes repo files', () => {
    const dir = join(sandbox(), 'repo');
    makeGitRepo(dir);
    writeRepoFile(dir, 'a.txt', 'one\n');
    expect(readFileSync(join(dir, 'a.txt'), 'utf8')).toBe('one\n');
    writeRepoFile(dir, 'a.txt', 'two\n');
    expect(readFileSync(join(dir, 'a.txt'), 'utf8')).toBe('two\n');
    removeRepoFile(dir, 'a.txt');
    expect(existsSync(join(dir, 'a.txt'))).toBe(false);
  });

  it('cleanup removes the tree and tolerates a missing one', () => {
    const dir = join(sandbox(), 'repo');
    makeGitRepo(dir);
    cleanupCorpus(dir);
    expect(existsSync(dir)).toBe(false);
    expect(() => cleanupCorpus(join(sandbox(), 'never-there'))).not.toThrow();
  });

  // A git hook in a linked worktree exports GIT_DIR/GIT_WORK_TREE; inherited,
  // they aimed the fixture's init/config/add/commit at the outer repo (it set
  // core.bare and a test identity there). The fixture must ignore them.
  it('builds its own repo even when the caller exports GIT_DIR', () => {
    const decoy = join(sandbox(), 'decoy');
    execFileSync('git', ['init', '-q', decoy], { env: cleanGitEnv() });
    const saved = { dir: process.env.GIT_DIR, tree: process.env.GIT_WORK_TREE };
    process.env.GIT_DIR = join(decoy, '.git');
    process.env.GIT_WORK_TREE = decoy;
    const dir = join(sandbox(), 'repo');
    try {
      makeGitRepo(dir, { 'a.txt': 'one\n' });
    } finally {
      restore('GIT_DIR', saved.dir);
      restore('GIT_WORK_TREE', saved.tree);
    }
    const log = (repo: string) =>
      execFileSync('git', ['-C', repo, 'log', '--format=%s'], {
        env: cleanGitEnv(),
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'ignore']
      }).trim();
    expect(log(dir)).toBe('init');
    expect(() => log(decoy)).toThrow();
    const decoyConfig = readFileSync(join(decoy, '.git', 'config'), 'utf8');
    expect(decoyConfig).not.toContain('test@sensei.test');
    expect(decoyConfig).toContain('bare = false');
  });
});

function cleanGitEnv(): NodeJS.ProcessEnv {
  return Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_')));
}

function restore(key: string, value: string | undefined) {
  if (value === undefined) delete process.env[key];
  else process.env[key] = value;
}
