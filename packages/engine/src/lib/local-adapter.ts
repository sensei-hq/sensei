import { readdir, readFile, access } from "fs/promises";
import { join, extname, basename, dirname } from "path";
import { fileURLToPath } from "url";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { extractSummary, parseLlmsIndex } from "./doc-utils.js";

export class LocalAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url?.startsWith("file://")) {
      throw new Error(`LocalAdapter: entry "${entry.name}" requires a file:// base_url`);
    }
    const dirPath = fileURLToPath(entry.base_url);

    const stat = await access(dirPath).then(() => true).catch(() => false);
    if (!stat) throw new Error(`LocalAdapter: path not found: ${dirPath}`);

    const llmsTxtPath = dirPath.endsWith("llms.txt")
      ? dirPath
      : join(dirPath, "llms.txt");

    const hasIndex = await access(llmsTxtPath).then(() => true).catch(() => false);
    if (hasIndex) return fetchFromIndex(llmsTxtPath);

    return walkDir(dirPath, dirPath);
  }
}

async function fetchFromIndex(llmsTxtPath: string): Promise<DocPage[]> {
  const text = await readFile(llmsTxtPath, "utf-8");
  const indexUrl = `file://${llmsTxtPath}`;
  const entries = parseLlmsIndex(text, indexUrl);
  const pages: DocPage[] = [];

  for (let i = 0; i < entries.length; i++) {
    const e = entries[i];
    const localPath = fileURLToPath(e.url);
    let content: string;
    try {
      content = await readFile(localPath, "utf-8");
    } catch (err) {
      console.warn(`[LocalAdapter] Failed to read ${localPath}:`, err instanceof Error ? err.message : String(err));
      content = e.summary;
    }
    pages.push({ title: e.title, localPath, summary: e.summary, content, sourceType: "local", component: e.component, sequence: i });
  }
  return pages;
}

async function walkDir(dir: string, rootPath: string): Promise<DocPage[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const pages: DocPage[] = [];

  for (const dirent of entries) {
    const fullPath = join(dir, dirent.name);
    if (dirent.isDirectory()) {
      pages.push(...await walkDir(fullPath, rootPath));
    } else if (dirent.isFile()) {
      const ext = extname(dirent.name).toLowerCase();
      if (ext !== ".md" && ext !== ".txt") continue;
      const content = await readFile(fullPath, "utf-8");
      const title = basename(dirent.name, ext);
      const summary = extractSummary(content);
      const parentDir = dirname(fullPath);
      const component = parentDir !== rootPath ? basename(parentDir) : undefined;
      pages.push({ title, localPath: fullPath, summary, content, sourceType: "local", component });
    }
  }
  return pages;
}
