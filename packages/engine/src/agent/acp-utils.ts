/**
 * Shared utilities for ACP adapter implementations.
 */

import { readFile, writeFile, mkdir } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname } from "path";

/**
 * Read a JSON config file, returning an empty object if missing or unparseable.
 */
export async function readJsonConfig(filePath: string): Promise<Record<string, unknown>> {
  if (!existsSync(filePath)) return {};
  try {
    return JSON.parse(await readFile(filePath, "utf-8"));
  } catch {
    return {};
  }
}

/**
 * Write a JSON config file, creating parent directories as needed.
 */
export async function writeJsonConfig(filePath: string, data: Record<string, unknown>): Promise<void> {
  await mkdir(dirname(filePath), { recursive: true });
  await writeFile(filePath, JSON.stringify(data, null, 2));
}

/**
 * Find the sensei binary on PATH. Prefers `senseid` (dedicated daemon), falls back to `sensei`.
 */
export function findSenseiBinary(): string {
  const PATH = process.env.PATH ?? "";
  for (const name of ["senseid", "sensei"]) {
    for (const dir of PATH.split(":")) {
      if (existsSync(join(dir, name))) return name;
    }
  }
  return "sensei";
}

/**
 * Merge incoming hook event arrays into an existing hooks config object.
 * Avoids duplicate entries by checking for existing sensei commands.
 *
 * @param existing  Current hooks object from settings (e.g. { "Stop": [...] })
 * @param incoming  New hooks to merge in (e.g. { "Stop": [{ matcher, hooks }] })
 */
export function mergeHooks(
  existing: Record<string, unknown>,
  incoming: Record<string, unknown[]>,
): Record<string, unknown> {
  const result = { ...existing };
  for (const [event, newEntries] of Object.entries(incoming)) {
    const current = (result[event] as unknown[]) ?? [];
    const filtered = (current as Array<{ matcher?: string; hooks?: Array<{ type: string; command: string }> }>)
      .filter(entry =>
        !JSON.stringify(entry).includes("sensei") && !JSON.stringify(entry).includes("senseid"),
      );
    result[event] = [...filtered, ...newEntries];
  }
  return result;
}
