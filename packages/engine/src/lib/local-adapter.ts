import { readdir, readFile } from "fs/promises";
import { join, extname, basename, dirname } from "path";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

export class LocalAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.local_path) throw new Error(`LocalAdapter: entry "${entry.name}" requires local_path`);
    return walkDir(entry.local_path, entry.local_path);
  }
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
      const description = content.slice(0, 200);
      const parentDir = dirname(fullPath);
      const component = parentDir !== rootPath ? basename(parentDir) : undefined;
      pages.push({ title, localPath: fullPath, description, content, sourceType: "local", component });
    }
  }
  return pages;
}
