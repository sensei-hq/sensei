import { readFile } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname } from "path";
import fg from "fast-glob";

export interface EntryPointCandidate {
  path: string;        // repo-relative source path
  inferredRole: string;
}

// ─── Public API ──────────────────────────────────────────────────────────────

export async function inferEntryPoints(repoPath: string): Promise<EntryPointCandidate[]> {
  const [fromJs, fromPy, fromGo, fromRust] = await Promise.all([
    inferFromPackageJson(repoPath),
    inferFromPyprojectToml(repoPath),
    inferFromGoConvention(repoPath),
    inferFromCargoToml(repoPath),
  ]);

  const all = [...fromJs, ...fromPy, ...fromGo, ...fromRust];
  // Deduplicate by path — first occurrence wins
  const seen = new Set<string>();
  return all.filter(c => {
    if (seen.has(c.path)) return false;
    seen.add(c.path);
    return true;
  });
}

// ─── TypeScript / JavaScript ─────────────────────────────────────────────────

async function inferFromPackageJson(repoPath: string): Promise<EntryPointCandidate[]> {
  const results: EntryPointCandidate[] = [];

  // Root package
  const rootPkg = await readJsonFile(join(repoPath, "package.json"));
  if (rootPkg) results.push(...extractFromPkg(repoPath, rootPkg, ""));

  // Workspace packages
  const workspaceGlobs: string[] = [];
  if (Array.isArray(rootPkg?.workspaces)) {
    for (const ws of rootPkg.workspaces as string[]) {
      workspaceGlobs.push(`${ws}/package.json`);
    }
  }
  if (workspaceGlobs.length > 0) {
    const wsPkgPaths = await fg(workspaceGlobs, { cwd: repoPath, ignore: ["**/node_modules/**"] });
    for (const relPkgPath of wsPkgPaths) {
      const pkg = await readJsonFile(join(repoPath, relPkgPath));
      if (pkg) {
        const pkgDir = dirname(relPkgPath); // e.g. "packages/cli"
        results.push(...extractFromPkg(repoPath, pkg, pkgDir));
      }
    }
  }

  return results;
}

function extractFromPkg(
  repoPath: string,
  pkg: Record<string, unknown>,
  pkgDir: string,
): EntryPointCandidate[] {
  const results: EntryPointCandidate[] = [];

  // bin field
  const bin = pkg.bin;
  if (bin && typeof bin === "object") {
    for (const [key, distPath] of Object.entries(bin as Record<string, string>)) {
      const resolved = resolveDistToSrc(repoPath, pkgDir, distPath);
      results.push({ path: resolved, inferredRole: `${key} CLI binary` });
    }
  }

  // main field
  if (typeof pkg.main === "string") {
    const resolved = resolveDistToSrc(repoPath, pkgDir, pkg.main);
    results.push({ path: resolved, inferredRole: "main entry" });
  }

  return results;
}

/**
 * Heuristic: dist/foo.js → src/foo.ts (if source exists), else keep dist path.
 */
function resolveDistToSrc(repoPath: string, pkgDir: string, distRelPath: string): string {
  const stripped = distRelPath.replace(/^\.\//, "");
  const repoRelDist = pkgDir ? `${pkgDir}/${stripped}` : stripped;

  const srcCandidate = repoRelDist.replace(/\bdist\//, "src/").replace(/\.js$/, ".ts");
  if (existsSync(join(repoPath, srcCandidate))) return srcCandidate;

  const tsxCandidate = repoRelDist.replace(/\bdist\//, "src/").replace(/\.js$/, ".tsx");
  if (existsSync(join(repoPath, tsxCandidate))) return tsxCandidate;

  return repoRelDist;
}

// ─── Python ──────────────────────────────────────────────────────────────────

async function inferFromPyprojectToml(repoPath: string): Promise<EntryPointCandidate[]> {
  const tomlPath = join(repoPath, "pyproject.toml");
  if (!existsSync(tomlPath)) return [];

  const content = await readFile(tomlPath, "utf-8");
  const results: EntryPointCandidate[] = [];

  let inScripts = false;
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "[project.scripts]") { inScripts = true; continue; }
    if (inScripts && trimmed.startsWith("[")) { inScripts = false; continue; }
    if (!inScripts) continue;

    const m = /^([\w-]+)\s*=\s*"([\w.]+):([\w]+)"/.exec(trimmed);
    if (!m) continue;
    const [, scriptName, modulePath] = m;
    const filePath = modulePath.replace(/\./g, "/") + ".py";
    if (!existsSync(join(repoPath, filePath))) continue;
    results.push({ path: filePath, inferredRole: `${scriptName} entry point` });
  }

  return results;
}

// ─── Go ──────────────────────────────────────────────────────────────────────

async function inferFromGoConvention(repoPath: string): Promise<EntryPointCandidate[]> {
  const mains = await fg(["cmd/*/main.go"], { cwd: repoPath });
  return mains.map(p => {
    const parts = p.split("/");
    const binName = parts[1] ?? "main";
    return { path: p, inferredRole: `${binName} binary` };
  });
}

// ─── Rust ────────────────────────────────────────────────────────────────────

async function inferFromCargoToml(repoPath: string): Promise<EntryPointCandidate[]> {
  const cargoPath = join(repoPath, "Cargo.toml");
  if (!existsSync(cargoPath)) return [];

  const content = await readFile(cargoPath, "utf-8");
  const results: EntryPointCandidate[] = [];

  let inBin = false;
  let name: string | null = null;
  let path: string | null = null;

  const flush = () => {
    if (path && existsSync(join(repoPath, path))) {
      results.push({ path, inferredRole: `${name ?? "binary"} binary` });
    }
    name = null;
    path = null;
  };

  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "[[bin]]") {
      flush();
      inBin = true;
      continue;
    }
    if (inBin && trimmed.startsWith("[[")) {
      flush();
      inBin = false;
      continue;
    }
    if (!inBin) continue;

    const namM = /^name\s*=\s*"([^"]+)"/.exec(trimmed);
    if (namM) { name = namM[1]; continue; }

    const pathM = /^path\s*=\s*"([^"]+)"/.exec(trimmed);
    if (pathM) { path = pathM[1]; }
  }
  flush();

  return results;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async function readJsonFile(absPath: string): Promise<Record<string, unknown> | null> {
  if (!existsSync(absPath)) return null;
  try {
    return JSON.parse(await readFile(absPath, "utf-8"));
  } catch {
    return null;
  }
}
