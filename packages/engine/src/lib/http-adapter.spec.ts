// packages/engine/src/lib/http-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { HttpAdapter } from "./http-adapter.js";
import type { LibEntry } from "@sensei/shared";

// Entry page HTML — links to sub-pages
const DOCS_HTML = `<!DOCTYPE html><html><body>
<nav>
  <a href="/docs/installation">Installation</a>
  <a href="/docs/usage">Usage</a>
  <a href="https://other.com/external">External (skip)</a>
  <a href="/blog/post">Blog (skip — wrong prefix)</a>
</nav>
<article><h1>Docs Home</h1><p>Welcome to the docs.</p></article>
</body></html>`;

const INSTALL_MD = `# Installation\n\nRun npm install mylib to get started.`;
const USAGE_MD = `# Usage\n\n## Quick Start\n\nImport and call init().\n\n## Advanced\n\nSee config options.`;

describe("HttpAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  function mockFetch(responses: Record<string, { body: string; type?: string }>) {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      const entry = responses[url];
      if (!entry) return Promise.resolve({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") });
      const type = entry.type ?? "text/html";
      return Promise.resolve({
        ok: true,
        headers: { get: (h: string) => h === "content-type" ? type : null },
        text: () => Promise.resolve(entry.body),
      });
    }));
  }

  it("discovers links with same path prefix and fetches each as a DocPage", async () => {
    mockFetch({
      "https://kavach.dev/docs": { body: DOCS_HTML },
      "https://kavach.dev/docs/installation": { body: INSTALL_MD, type: "text/markdown" },
      "https://kavach.dev/docs/usage": { body: USAGE_MD, type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev/docs" };
    const pages = await adapter.fetch(entry);

    // Entry URL + discovered sub-pages
    expect(pages.length).toBeGreaterThanOrEqual(2);
    const urls = pages.map(p => p.url);
    expect(urls).toContain("https://kavach.dev/docs/installation");
    expect(urls).toContain("https://kavach.dev/docs/usage");
    // External and off-prefix links are excluded
    expect(urls).not.toContain("https://other.com/external");
    expect(urls).not.toContain("https://kavach.dev/blog/post");
    pages.forEach(p => {
      expect(p.sourceType).toBe("http");
      expect(p.content).toBeTruthy();
      expect(p.summary.length).toBeLessThanOrEqual(200);
    });
  });

  it("assigns component from first path segment after base", async () => {
    const html = `<html><body><a href="/docs/hooks/use-auth">UseAuth</a></body></html>`;
    mockFetch({
      "https://example.com/docs": { body: html },
      "https://example.com/docs/hooks/use-auth": { body: "# UseAuth\n\nAuth hook.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "http", base_url: "https://example.com/docs" });

    const hookPage = pages.find(p => p.url?.includes("use-auth"));
    expect(hookPage?.component).toBe("hooks");
    // Entry URL itself has no component
    const entryPage = pages.find(p => p.url === "https://example.com/docs");
    expect(entryPage?.component).toBeUndefined();
  });

  it("deduplicates URLs", async () => {
    // Same link appears twice in the page
    const html = `<html><body><a href="/docs/page">P1</a><a href="/docs/page">P1 again</a></body></html>`;
    mockFetch({
      "https://x.com/docs": { body: html },
      "https://x.com/docs/page": { body: "# Page\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const withPage = pages.filter(p => p.url?.includes("/page"));
    expect(withPage).toHaveLength(1);
  });

  it("includes entry URL at sequence 0 even if not linked", async () => {
    const html = `<html><body><article><h1>Home</h1><p>Home page.</p></article><a href="/docs/sub">Sub</a></body></html>`;
    mockFetch({
      "https://x.com/docs": { body: html },
      "https://x.com/docs/sub": { body: "# Sub\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    expect(pages[0].url).toBe("https://x.com/docs");
    expect(pages[0].sequence).toBe(0);
  });

  it("uses sitemap.xml to discover pages when available", async () => {
    const sitemap = `<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://x.com/docs</loc></url>
  <url><loc>https://x.com/docs/page-a</loc></url>
  <url><loc>https://x.com/docs/page-b</loc></url>
  <url><loc>https://x.com/about</loc></url>
</urlset>`;
    mockFetch({
      "https://x.com/sitemap.xml": { body: sitemap, type: "application/xml" },
      "https://x.com/docs": { body: `<html><body><h1>Home</h1></body></html>` },
      "https://x.com/docs/page-a": { body: "# Page A\n\nContent A.", type: "text/markdown" },
      "https://x.com/docs/page-b": { body: "# Page B\n\nContent B.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const urls = pages.map(p => p.url);
    expect(urls).toContain("https://x.com/docs/page-a");
    expect(urls).toContain("https://x.com/docs/page-b");
    // /about is outside the /docs prefix — must be excluded
    expect(urls).not.toContain("https://x.com/about");
  });

  it("falls back to HTML link extraction when sitemap is absent", async () => {
    mockFetch({
      // No sitemap entry — 404
      "https://x.com/docs": { body: `<html><body><a href="/docs/page">Page</a></body></html>` },
      "https://x.com/docs/page": { body: "# Page\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    expect(pages.map(p => p.url)).toContain("https://x.com/docs/page");
  });

  it("strips URL fragments to avoid fetching the same page twice", async () => {
    // Sidebar links often include fragment anchors like /docs/adapters/supabase#config
    const html = `<html><body>
      <a href="/docs/page">Page</a>
      <a href="/docs/page#section-1">Page § 1</a>
      <a href="/docs/page#section-2">Page § 2</a>
    </body></html>`;
    mockFetch({
      "https://x.com/docs": { body: html },
      "https://x.com/docs/page": { body: "# Page\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const pagePaths = pages.filter(p => p.url?.endsWith("/page"));
    expect(pagePaths).toHaveLength(1);
  });

  it("does not follow links outside the entry URL subtree", async () => {
    // /documentation starts with /docs but is NOT under /docs/
    const html = `<html><body>
      <a href="/docs/sub">Sub</a>
      <a href="/documentation/other">Other (wrong prefix)</a>
    </body></html>`;
    mockFetch({
      "https://x.com/docs": { body: html },
      "https://x.com/docs/sub": { body: "# Sub\n\nContent.", type: "text/markdown" },
    });

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const urls = pages.map(p => p.url);
    expect(urls).toContain("https://x.com/docs/sub");
    expect(urls).not.toContain("https://x.com/documentation/other");
  });

  it("returns empty pages when entry URL fetch returns non-ok status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") }));
    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    expect(pages).toHaveLength(0);
  });

  it("skips sub-pages that fail to fetch (graceful degradation)", async () => {
    const html = `<html><body><a href="/docs/ok">OK</a><a href="/docs/broken">Broken</a></body></html>`;
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url === "https://x.com/docs") return Promise.resolve({ ok: true, headers: { get: () => "text/html" }, text: () => Promise.resolve(html) });
      if (url === "https://x.com/docs/ok") return Promise.resolve({ ok: true, headers: { get: () => "text/markdown" }, text: () => Promise.resolve("# OK\n\nContent.") });
      return Promise.resolve({ ok: false, status: 500, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com/docs" });
    const urls = pages.map(p => p.url);
    expect(urls).toContain("https://x.com/docs/ok");
    expect(urls).not.toContain("https://x.com/docs/broken");
  });
});
