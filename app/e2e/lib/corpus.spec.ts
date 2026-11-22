import { describe, it, expect, afterEach } from 'vitest';
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
});
