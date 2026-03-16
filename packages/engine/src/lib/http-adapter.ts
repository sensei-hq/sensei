// packages/engine/src/lib/http-adapter.ts
import { Readability } from "@mozilla/readability";
import { JSDOM } from "jsdom";
import TurndownService from "turndown";
import type { LibEntry, DocPage } from "@sensei/shared";
import type { SourceAdapter } from "./source-adapter.js";
import { resolveUrl, extractSummary } from "./doc-utils.js";

const MAX_PAGES = 100;
const td = new TurndownService({ headingStyle: "atx" });

export class HttpAdapter implements SourceAdapter {
  async fetch(entry: LibEntry): Promise<DocPage[]> {
    if (!entry.base_url) throw new Error(`HttpAdapter: entry "${entry.name}" requires base_url`);
    const entryUrl = entry.base_url;

    const pages: DocPage[] = [];
    const seen = new Set<string>([entryUrl]);
    const queue: string[] = [entryUrl];

    // Pre-populate queue from sitemap when available — works even for JS-rendered sites
    for (const url of await fetchSitemapUrls(entryUrl)) {
      if (!seen.has(url)) { seen.add(url); queue.push(url); }
    }

    while (queue.length > 0 && pages.length < MAX_PAGES) {
      const url = queue.shift()!;

      let body: string;
      let contentType: string;
      try {
        const res = await fetch(url);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        body = await res.text();
        contentType = res.headers.get("content-type") ?? "";
      } catch (err) {
        console.warn(`[HttpAdapter] Failed to fetch ${url}:`, err instanceof Error ? err.message : String(err));
        continue;
      }

      const markdown = bodyToMarkdown(body, url, contentType);
      pages.push(makeDocPage(markdown, url, entryUrl, pages.length));

      // Discover new links from this page and enqueue them (BFS)
      for (const link of extractLinks(body, url, entryUrl, seen)) {
        seen.add(link);
        queue.push(link);
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

function extractLinks(html: string, pageUrl: string, entryUrl: string, seen: Set<string>): string[] {
  const dom = new JSDOM(html, { url: pageUrl });
  // Normalize: ensure prefix ends with "/" so "/docs" doesn't match "/documentation"
  const prefix = entryUrl.endsWith("/") ? entryUrl : entryUrl + "/";
  const links: string[] = [];

  for (const a of Array.from(dom.window.document.querySelectorAll("a[href]"))) {
    const href = (a as HTMLAnchorElement).getAttribute("href");
    if (!href) continue;
    let absolute: string;
    try {
      absolute = resolveUrl(pageUrl, href);
    } catch {
      continue;
    }
    // Strip fragment — /page#section and /page are the same document
    const withoutFragment = absolute.split("#")[0];
    // Keep only URLs under the entry subtree (full URL prefix, not just path)
    if (withoutFragment !== entryUrl && !withoutFragment.startsWith(prefix)) continue;
    if (seen.has(withoutFragment)) continue;
    seen.add(withoutFragment);
    links.push(withoutFragment);
  }
  return links;
}

async function fetchSitemapUrls(entryUrl: string): Promise<string[]> {
  const sitemapUrl = new URL("/sitemap.xml", entryUrl).href;
  try {
    const res = await fetch(sitemapUrl);
    if (!res.ok) return [];
    const xml = await res.text();
    const prefix = entryUrl.endsWith("/") ? entryUrl : entryUrl + "/";
    const urls: string[] = [];
    for (const match of xml.matchAll(/<loc>([^<]+)<\/loc>/g)) {
      const url = match[1].trim();
      if (url === entryUrl || url.startsWith(prefix)) urls.push(url);
    }
    if (urls.length > 0) console.info(`[HttpAdapter] Sitemap: found ${urls.length} URLs under ${entryUrl}`);
    return urls;
  } catch {
    return [];
  }
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
