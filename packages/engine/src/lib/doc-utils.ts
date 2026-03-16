// packages/engine/src/lib/doc-utils.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";

const td = new TurndownService({ headingStyle: "atx" });

/** Resolve a relative URL against a base URL. */
export function resolveUrl(base: string, relative: string): string {
  return new URL(relative, base).href;
}

/**
 * Fetch a URL and return its content as Markdown.
 * - .md extension or text/plain|text/markdown content-type → returned as-is
 * - Otherwise → Readability + Turndown (HTML → Markdown)
 * Throws on network or HTTP error. Callers handle gracefully.
 */
export async function fetchAsMarkdown(url: string): Promise<string> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`fetchAsMarkdown: HTTP ${res.status} for ${url}`);

  const contentType = res.headers.get("content-type") ?? "";
  const body = await res.text();

  const isMarkdown =
    url.endsWith(".md") ||
    contentType.includes("text/plain") ||
    contentType.includes("text/markdown");

  if (isMarkdown) return body;

  const dom = new JSDOM(body, { url });
  const reader = new Readability(dom.window.document);
  const article = reader.parse();
  return td.turndown(article?.content ?? body);
}

/** First non-heading paragraph, trimmed to ≤200 chars. */
export function extractSummary(markdown: string): string {
  const lines = markdown.split("\n");
  const buf: string[] = [];
  let inParagraph = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("#")) {
      if (inParagraph) break;
      continue;
    }
    if (trimmed === "") {
      if (inParagraph) break;
      continue;
    }
    inParagraph = true;
    buf.push(trimmed);
  }

  return buf.join(" ").slice(0, 200);
}

/**
 * Split markdown at H2 headings into sections.
 * - Content before first ## → title "Overview" (omit if empty after trim)
 * - Each ## heading → one section (includes all H3+ sub-content within it)
 * - sequence is 0-based index within THIS document (independent per document)
 * - Sections with empty content after whitespace trim are omitted.
 */
export function splitSections(
  markdown: string,
): Array<{ title: string; content: string; sequence: number }> {
  if (!markdown.trim()) return [];

  const blocks = markdown.split(/\n(?=## )/);
  const result: Array<{ title: string; content: string; sequence: number }> = [];
  let sequence = 0;

  for (const block of blocks) {
    const trimmed = block.trim();
    if (!trimmed) continue;

    if (trimmed.startsWith("## ")) {
      const lines = trimmed.split("\n");
      const title = lines[0].replace(/^##\s+/, "").trim();
      const content = lines.slice(1).join("\n").trim();
      if (!content) continue;
      result.push({ title, content, sequence: sequence++ });
    } else {
      // Pre-H2 content → Overview (strip leading H1)
      const withoutH1 = trimmed.replace(/^#[^#][^\n]*\n/, "").trim();
      if (!withoutH1) continue;
      result.push({ title: "Overview", content: withoutH1, sequence: sequence++ });
    }
  }

  return result;
}
