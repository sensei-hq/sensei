// packages/engine/src/lib/llms-txt-adapter.ts
import { readFile } from "fs/promises";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

const LINK_RE = /^-\s+\[([^\]]+)\]\(([^)]+)\):\s*(.+)$/;
const SECTION_RE = /^##\s+(.+)$/;

export class LlmsTxtAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    let text: string;
    if (entry.base_url?.startsWith("http")) {
      const res = await fetch(entry.base_url);
      if (!res.ok) throw new Error(`LlmsTxtAdapter: fetch failed for ${entry.base_url}: ${res.status}`);
      text = await res.text();
    } else if (entry.local_path) {
      text = await readFile(entry.local_path, "utf-8");
    } else {
      throw new Error(`LlmsTxtAdapter: entry "${entry.name}" must have base_url or local_path`);
    }
    return parse(text);
  }
}

function parse(text: string): DocPage[] {
  const pages: DocPage[] = [];
  let currentSection: string | undefined;

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    const sectionMatch = line.match(SECTION_RE);
    if (sectionMatch) {
      currentSection = sectionMatch[1].trim();
      continue;
    }
    const linkMatch = line.match(LINK_RE);
    if (!linkMatch) continue;
    const [, title, url, description] = linkMatch;
    pages.push({ title: title.trim(), url: url.trim(), description: description.trim(), sourceType: "llms.txt", component: currentSection });
  }
  return pages;
}
