import { readFile } from "fs/promises";
import { join } from "path";

/**
 * Reads a file and returns its contents as a string.
 */
export async function readTextFile(filePath: string): Promise<string> {
  const contents = await readFile(filePath, "utf-8");
  return contents.trim();
}

export class FileCache {
  private cache: Map<string, string> = new Map();

  get(key: string): string | undefined {
    return this.cache.get(key);
  }

  set(key: string, value: string): void {
    this.cache.set(key, value);
  }
}

export interface CacheOptions {
  ttl: number;
  maxSize: number;
}

export type CacheKey = string;

const DEFAULT_TTL = 60_000;

function resolveFilePath(base: string, name: string): string {
  return join(base, name);
}
