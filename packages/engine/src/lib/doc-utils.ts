// packages/engine/src/lib/doc-utils.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";

const td = new TurndownService({ headingStyle: "atx" });

/** Resolve a relative URL against a base URL. Accepts file:// and http(s):// bases. */
export function resolveUrl(base: string, relative: string): string {
  return new URL(relative, base).href;
}

/** Convert a GitHub blob URL to its raw.githubusercontent.com equivalent. */
export function toRawGithubUrl(url: string): string {
  const m = url.match(/^https:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\/(.+)$/);
  if (m) return `https://raw.githubusercontent.com/${m[1]}/${m[2]}/${m[3]}/${m[4]}`;
  return url;
}

// ─── llms.txt index parsing ──────────────────────────────────────────────────

const LLMS_LINK_RE = /^-\s+\[([^\]]+)\]\(([^)]+)\)[:\s—–-]+\s*(.+)$/;
const LLMS_SECTION_RE = /^##\s+(.+)$/;

export interface LlmsIndexEntry {
  title: string;
  /** Absolute URL (http/https or file://) */
  url: string;
  summary: string;
  component?: string;
}

/**
 * Parse an llms.txt index file into a list of entries.
 * @param text   Raw text of the llms.txt file
 * @param baseUrl Absolute URL (http/https or file://) used to resolve relative links
 */
export function parseLlmsIndex(text: string, baseUrl: string): LlmsIndexEntry[] {
  const entries: LlmsIndexEntry[] = [];
  let currentSection: string | undefined;

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    const sectionMatch = line.match(LLMS_SECTION_RE);
    if (sectionMatch) { currentSection = sectionMatch[1].trim(); continue; }
    const linkMatch = line.match(LLMS_LINK_RE);
    if (!linkMatch) continue;
    const [, title, rel, summary] = linkMatch;
    entries.push({
      title: title.trim(),
      url: resolveUrl(baseUrl, rel.trim()),
      summary: summary.trim(),
      component: currentSection,
    });
  }
  return entries;
}

/**
 * Fetch a URL and return its content as Markdown.
 * - .md / .txt extension or text/plain|text/markdown content-type → returned as-is
 * - GitHub blob URLs → redirected to raw.githubusercontent.com automatically
 * - Otherwise → Readability + Turndown (HTML → Markdown)
 * Throws on network or HTTP error. Callers handle gracefully.
 */
export async function fetchAsMarkdown(url: string): Promise<string> {
  const fetchUrl = toRawGithubUrl(url);
  const res = await fetch(fetchUrl);
  if (!res.ok) throw new Error(`fetchAsMarkdown: HTTP ${res.status} for ${fetchUrl}`);

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
