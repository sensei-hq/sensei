// packages/engine/src/lib/llms-txt-adapter.ts
import { readFile } from "fs/promises";
import { fileURLToPath } from "url";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { fetchAsMarkdown, parseLlmsIndex } from "./doc-utils.js";

export class LlmsTxtAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`LlmsTxtAdapter: entry "${entry.name}" requires base_url`);

    let text: string;
    if (entry.base_url.startsWith("file://")) {
      text = await readFile(fileURLToPath(entry.base_url), "utf-8");
    } else {
      const res = await fetch(entry.base_url);
      if (!res.ok) throw new Error(`LlmsTxtAdapter: fetch failed for ${entry.base_url}: ${res.status}`);
      text = await res.text();
    }

    const entries = parseLlmsIndex(text, entry.base_url);
    const pages: DocPage[] = [];

    for (let i = 0; i < entries.length; i++) {
      const e = entries[i];
      let content: string;
      try {
        content = e.url.startsWith("file://")
          ? await readFile(fileURLToPath(e.url), "utf-8")
          : await fetchAsMarkdown(e.url);
      } catch (err) {
        console.warn(`[LlmsTxtAdapter] Failed to fetch ${e.url}:`, err instanceof Error ? err.message : String(err));
        content = e.summary;
      }
      const isLocal = e.url.startsWith("file://");
      pages.push({
        title: e.title,
        url: isLocal ? undefined : e.url,
        localPath: isLocal ? fileURLToPath(e.url) : undefined,
        summary: e.summary,
        content,
        sourceType: "llms.txt",
        component: e.component,
        sequence: i,
      });
    }
    return pages;
  }
}
