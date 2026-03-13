import glob from "fast-glob";
import { createHash } from "crypto";
import { readFile, stat } from "fs/promises";
import { join, relative } from "path";
import type { FileEntry, ScanResult } from "@sensei/shared";

const DEFAULT_EXCLUDE = [
  "**/node_modules/**",
  "**/.git/**",
  "**/dist/**",
  "**/build/**",
  "**/.sensei/**",
  "**/__pycache__/**",
  "**/.venv/**",
];

const DEFAULT_EXTENSIONS = [
  "**/*.ts",
  "**/*.tsx",
  "**/*.js",
  "**/*.jsx",
  "**/*.mjs",
  "**/*.cjs",
  "**/*.py",
  "**/*.go",
  "**/*.rs",
  "**/*.md",
  "**/*.yaml",
  "**/*.yml",
  "**/*.json",
  "**/*.toml",
];

export interface PriorFileState {
  file_path: string;
  mtime: number;
  content_hash: string;
}

export interface ScannerOptions {
  repoPath: string;
  repoId: string;
  include?: string[];
  exclude?: string[];
  priorState?: PriorFileState[];
}

export class Scanner {
  constructor(private opts: ScannerOptions) {}

  async scan(): Promise<ScanResult> {
    const { repoPath, repoId } = this.opts;
    const include = this.opts.include ?? DEFAULT_EXTENSIONS;
    const exclude = this.opts.exclude ?? DEFAULT_EXCLUDE;

    const absPaths = await glob(include, {
      cwd: repoPath,
      ignore: exclude,
      absolute: true,
      followSymbolicLinks: false,
    });

    const files: FileEntry[] = await Promise.all(
      absPaths.map(abs => this.fingerprint(abs, repoPath))
    );

    const priorByPath = new Map<string, PriorFileState>(
      (this.opts.priorState ?? []).map(s => [s.file_path, s])
    );

    const currentPaths = new Set(files.map(f => f.path));
    const priorPaths = new Set(priorByPath.keys());

    const changed = files
      .filter(f => {
        const prior = priorByPath.get(f.path);
        if (!prior) return true;                         // new file
        if (prior.mtime !== f.mtime) {
          return prior.content_hash !== f.hash;          // mtime changed → check hash
        }
        return false;                                    // unchanged
      })
      .map(f => f.path);

    const deleted = [...priorPaths].filter(p => !currentPaths.has(p));

    return { repoId, files, changed, deleted };
  }

  private async fingerprint(absPath: string, repoPath: string): Promise<FileEntry> {
    const [stats, contents] = await Promise.all([
      stat(absPath),
      readFile(absPath),
    ]);
    const hash = createHash("sha256").update(contents).digest("hex");
    return {
      path: relative(repoPath, absPath),
      absPath,
      mtime: stats.mtimeMs,
      hash,
      size: stats.size,
    };
  }
}
