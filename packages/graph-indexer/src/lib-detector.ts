/**
 * Detects external library usage from parsed imports.
 *
 * Groups org-scoped packages (@rokkit/ui, @rokkit/core → "rokkit").
 * Distinguishes workspace-internal packages from external deps.
 */
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { existsSync } from "node:fs";

/** Detect workspace packages from package.json workspaces config. */
export async function detectWorkspacePackages(repoPath: string): Promise<Set<string>> {
  const internal = new Set<string>();

  // Node.js / Bun workspaces
  const pkgPath = join(repoPath, "package.json");
  if (existsSync(pkgPath)) {
    try {
      const pkg = JSON.parse(await readFile(pkgPath, "utf-8")) as {
        name?: string;
        workspaces?: string[] | { packages?: string[] };
      };
      // The root package itself
      if (pkg.name) internal.add(pkg.name);

      // Scan workspace package.json files for their names
      const fg = (await import("fast-glob")).default;
      const patterns = Array.isArray(pkg.workspaces)
        ? pkg.workspaces
        : pkg.workspaces?.packages ?? [];

      if (patterns.length > 0) {
        const dirs = await fg(patterns.map(p => `${p}/package.json`), {
          cwd: repoPath,
          absolute: true,
        });
        for (const dir of dirs) {
          try {
            const sub = JSON.parse(await readFile(dir, "utf-8")) as { name?: string };
            if (sub.name) internal.add(sub.name);
          } catch { /* skip */ }
        }
      }
    } catch { /* not a valid package.json */ }
  }

  // Cargo.toml workspaces
  const cargoPath = join(repoPath, "Cargo.toml");
  if (existsSync(cargoPath)) {
    try {
      const cargo = await readFile(cargoPath, "utf-8");
      // Simple regex to find [workspace] members
      const membersMatch = cargo.match(/members\s*=\s*\[([\s\S]*?)\]/);
      if (membersMatch) {
        const members = membersMatch[1].match(/"([^"]+)"/g);
        if (members) {
          for (const m of members) {
            const memberPath = m.replace(/"/g, "");
            const memberCargoPath = join(repoPath, memberPath, "Cargo.toml");
            if (existsSync(memberCargoPath)) {
              try {
                const memberCargo = await readFile(memberCargoPath, "utf-8");
                const nameMatch = memberCargo.match(/^name\s*=\s*"([^"]+)"/m);
                if (nameMatch) internal.add(nameMatch[1]);
              } catch { /* skip */ }
            }
          }
        }
      }
    } catch { /* skip */ }
  }

  return internal;
}

/**
 * Normalize a package import to a lib name.
 * @rokkit/ui → "rokkit"
 * @sensei/engine → "sensei"
 * hono → "hono"
 * hono/context → "hono"
 */
export function importToLibName(importPath: string): string {
  if (importPath.startsWith("@")) {
    // @scope/package → scope (without @)
    const parts = importPath.split("/");
    return parts[0].slice(1); // remove @
  }
  // bare package: hono, zod, express → take first segment
  return importPath.split("/")[0];
}

/**
 * Collect external lib references from parsed imports.
 * Returns a map of lib name → set of specific packages used.
 */
export function collectExternalLibs(
  allImports: Array<{ targetPath: string; names: string[] }>,
  internalPackages: Set<string>,
): Map<string, Set<string>> {
  const libs = new Map<string, Set<string>>();

  for (const imp of allImports) {
    const target = imp.targetPath;

    // Skip relative imports
    if (target.startsWith(".") || target.startsWith("/")) continue;

    // Skip Node.js builtins
    if (target.startsWith("node:") || NODE_BUILTINS.has(target)) continue;

    // Check if this is a workspace-internal package
    const fullPkg = target.startsWith("@")
      ? target.split("/").slice(0, 2).join("/")
      : target.split("/")[0];

    if (internalPackages.has(fullPkg)) continue;

    // Group by org scope
    const libName = importToLibName(target);

    const set = libs.get(libName) ?? new Set();
    set.add(fullPkg);
    libs.set(libName, set);
  }

  return libs;
}

const NODE_BUILTINS = new Set([
  "assert", "buffer", "child_process", "cluster", "console", "constants",
  "crypto", "dgram", "dns", "domain", "events", "fs", "http", "https",
  "module", "net", "os", "path", "perf_hooks", "process", "punycode",
  "querystring", "readline", "repl", "stream", "string_decoder", "sys",
  "timers", "tls", "tty", "url", "util", "v8", "vm", "worker_threads", "zlib",
]);
