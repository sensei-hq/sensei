// packages/cli/src/lib/detect-libs.ts
import { readFile } from "fs/promises";
import { join } from "path";
import type { LibEntry } from "@sensei/shared";

/** Scan direct dependencies from package.json / requirements.txt / go.mod. */
export async function scanDirectDeps(cwd: string): Promise<string[]> {
  const deps: string[] = [];

  // Node.js — direct deps only, skip @types/*
  try {
    const pkg = JSON.parse(await readFile(join(cwd, "package.json"), "utf-8"));
    const direct = Object.keys(pkg.dependencies ?? {});
    deps.push(...direct.filter(d => !d.startsWith("@types/")));
  } catch { /* no package.json */ }

  // Python
  try {
    const reqs = await readFile(join(cwd, "requirements.txt"), "utf-8");
    const names = reqs
      .split("\n")
      .map(l => l.trim().split(/[=><!\[;]/)[0].trim())
      .filter(Boolean)
      .filter(l => !l.startsWith("#"));
    deps.push(...names);
  } catch { /* no requirements.txt */ }

  // Go
  try {
    const gomod = await readFile(join(cwd, "go.mod"), "utf-8");
    const block = gomod.match(/require\s*\(([^)]+)\)/s)?.[1] ?? "";
    const names = block
      .split("\n")
      .map(l => l.trim().split(/\s/)[0])
      .filter(Boolean)
      .filter(l => l !== "//" && !l.startsWith("//"));
    deps.push(...names);
  } catch { /* no go.mod */ }

  return deps;
}

/**
 * Infer source_type and URL/path fields from a user-provided string.
 * Evaluation order: llms.txt URL → any other HTTP URL → local path.
 */
export function inferSourceType(input: string): Pick<LibEntry, "source_type" | "base_url" | "local_path"> {
  if (input.startsWith("http://") || input.startsWith("https://")) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith("/llms.txt") || url.pathname === "/llms.txt") {
        return { source_type: "llms.txt", base_url: input };
      }
    } catch { /* malformed URL — fall through to local */ }
    return { source_type: "http", base_url: input };
  }
  return { source_type: "local", local_path: input };
}
