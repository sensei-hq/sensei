// packages/engine/src/lib/llms-txt-adapter.ts
import { readFile } from "fs/promises";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { resolveUrl, fetchAsMarkdown } from "./doc-utils.js";

const LINK_RE = /^-\s+\[([^\]]+)\]\(([^)]+)\):\s*(.+)$/;
const SECTION_RE = /^##\s+(.+)$/;

interface IndexEntry { title: string; url: string; summary: string; component?: string; }

export class LlmsTxtAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    let text: string;
    const indexUrl = entry.base_url ?? entry.local_path;

    if (entry.base_url?.startsWith("http")) {
      const res = await fetch(entry.base_url);
      if (!res.ok) throw new Error(`LlmsTxtAdapter: fetch failed for ${entry.base_url}: ${res.status}`);
      text = await res.text();
    } else if (entry.local_path) {
      text = await readFile(entry.local_path, "utf-8");
    } else {
      throw new Error(`LlmsTxtAdapter: entry "${entry.name}" must have base_url or local_path`);
    }

    const entries = parseIndex(text, indexUrl!);
    const pages: DocPage[] = [];

    for (let i = 0; i < entries.length; i++) {
      const e = entries[i];
      let content: string;
      try {
        content = await fetchAsMarkdown(e.url);
      } catch (err) {
        console.warn(`[LlmsTxtAdapter] Failed to fetch ${e.url}:`, err instanceof Error ? err.message : String(err));
        content = e.summary; // graceful degradation
      }
      pages.push({
        title: e.title,
        url: e.url,
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

function parseIndex(text: string, indexUrl: string): IndexEntry[] {
  const entries: IndexEntry[] = [];
  let currentSection: string | undefined;

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    const sectionMatch = line.match(SECTION_RE);
    if (sectionMatch) { currentSection = sectionMatch[1].trim(); continue; }
    const linkMatch = line.match(LINK_RE);
    if (!linkMatch) continue;
    const [, title, url, summary] = linkMatch;
    entries.push({
      title: title.trim(),
      url: resolveUrl(indexUrl, url.trim()),
      summary: summary.trim(),
      component: currentSection,
    });
  }
  return entries;
}
