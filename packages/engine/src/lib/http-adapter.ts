// packages/engine/src/lib/http-adapter.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";

const td = new TurndownService({ headingStyle: "atx" });

export class HttpAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`HttpAdapter: entry "${entry.name}" requires base_url`);

    const res = await fetch(entry.base_url);
    if (!res.ok) throw new Error(`HttpAdapter: fetch failed for ${entry.base_url}: ${res.status}`);

    const html = await res.text();
    const dom = new JSDOM(html, { url: entry.base_url });
    const reader = new Readability(dom.window.document);
    const article = reader.parse();
    const markdown = td.turndown(article?.content ?? html);

    return splitIntoPages(markdown, entry.base_url);
  }
}

function splitIntoPages(markdown: string, url: string): DocPage[] {
  const sections = markdown.split(/\n(?=## )/);

  if (sections.length <= 1) {
    const content = markdown.trim();
    return [{ title: "Overview", url, description: content.slice(0, 200), content, sourceType: "http" }];
  }

  return sections
    .map(s => s.trim())
    .filter(Boolean)
    .map(section => {
      const lines = section.split("\n");
      const title = lines[0].replace(/^##\s+/, "").trim();
      const body = lines.slice(1).join(" ").trim();
      return { title, url, description: body.slice(0, 200) || section.slice(0, 200), content: section.trim(), sourceType: "http" as const };
    });
}
