// packages/engine/src/lib/http-adapter.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { resolveUrl, fetchAsMarkdown, extractSummary } from "./doc-utils.js";

const MAX_PAGES = 100;
const td = new TurndownService({ headingStyle: "atx" });

export class HttpAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`HttpAdapter: entry "${entry.name}" requires base_url`);
    const entryUrl = entry.base_url;

    // Phase 1: Discover — fetch entry URL once, use body for both discovery and entry DocPage
    const entryRes = await fetch(entryUrl);
    if (!entryRes.ok) throw new Error(`HttpAdapter: HTTP ${entryRes.status} for ${entryUrl}`);
    const entryBody = await entryRes.text();
    const entryContentType = entryRes.headers.get("content-type") ?? "";

    const discovered = discoverLinks(entryBody, entryUrl);

    // Phase 2: Build DocPages
    const pages: DocPage[] = [];

    // Entry URL is always sequence 0 — convert already-fetched body (no second fetch)
    try {
      const entryMarkdown = bodyToMarkdown(entryBody, entryUrl, entryContentType);
      pages.push(makeDocPage(entryMarkdown, entryUrl, entryUrl, 0));
    } catch (err) {
      console.warn(`[HttpAdapter] Failed to convert entry ${entryUrl}:`, err instanceof Error ? err.message : String(err));
    }

    // Fetch each discovered sub-page
    for (const url of discovered) {
      if (pages.length >= MAX_PAGES) break;
      try {
        const res = await fetch(url);
        if (!res.ok) throw new Error(`HTTP ${res.status} for ${url}`);
        const body = await res.text();
        const ct = res.headers.get("content-type") ?? "";
        const markdown = bodyToMarkdown(body, url, ct);
        pages.push(makeDocPage(markdown, url, entryUrl, pages.length));
      } catch (err) {
        console.warn(`[HttpAdapter] Failed to fetch ${url}:`, err instanceof Error ? err.message : String(err));
      }
    }

    return pages;
  }
}

function bodyToMarkdown(body: string, url: string, contentType: string): string {
  const isMarkdown =
    url.endsWith(".md") ||
    contentType.includes("text/plain") ||
    contentType.includes("text/markdown");
  if (isMarkdown) return body;
  const dom = new JSDOM(body, { url });
  const reader = new Readability(dom.window.document);
  return td.turndown(reader.parse()?.content ?? body);
}

function discoverLinks(html: string, entryUrl: string): string[] {
  const dom = new JSDOM(html, { url: entryUrl });
  const baseUrl = new URL(entryUrl);
  const basePath = baseUrl.pathname;

  const seen = new Set<string>([entryUrl]);
  const links: string[] = [];

  for (const a of Array.from(dom.window.document.querySelectorAll("a[href]"))) {
    const href = (a as HTMLAnchorElement).getAttribute("href");
    if (!href) continue;
    let absolute: string;
    try {
      absolute = resolveUrl(entryUrl, href);
    } catch {
      continue;
    }
    const parsed = new URL(absolute);
    // Must be same origin, same path prefix, not already seen
    if (parsed.hostname !== baseUrl.hostname) continue;
    if (!parsed.pathname.startsWith(basePath)) continue;
    if (seen.has(absolute)) continue;
    seen.add(absolute);
    links.push(absolute);
    if (links.length >= MAX_PAGES - 1) break;
  }
  return links;
}

function makeDocPage(markdown: string, url: string, entryUrl: string, sequence: number): DocPage {
  const summary = extractSummary(markdown);
  const title = extractTitle(markdown) ?? new URL(url).pathname.split("/").filter(Boolean).pop() ?? "Page";
  const component = inferComponent(url, entryUrl);
  return { title, url, summary, content: markdown, sourceType: "http", component, sequence };
}

function extractTitle(markdown: string): string | undefined {
  return markdown.match(/^#\s+(.+)$/m)?.[1]?.trim();
}

function inferComponent(url: string, entryUrl: string): string | undefined {
  const basePath = new URL(entryUrl).pathname;
  const path = new URL(url).pathname;
  if (path === basePath) return undefined;
  const relative = path.slice(basePath.length).replace(/^\//, "");
  const segment = relative.split("/")[0];
  return segment || undefined;
}
