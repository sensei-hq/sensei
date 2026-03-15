// packages/engine/src/lib/http-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { HttpAdapter } from "./http-adapter.js";
import type { LibEntry } from "@sensei/shared";

const MULTI_SECTION_HTML = `<!DOCTYPE html><html><head><title>Kavach</title></head>
<body><article>
  <h1>Kavach Auth</h1><p>Zero-trust auth library.</p>
  <h2>Installation</h2><p>Run npm install kavach to get started.</p>
  <h2>Usage</h2><p>Import the module and call createClient with your config.</p>
</article></body></html>`;

const SINGLE_SECTION_HTML = `<!DOCTYPE html><html><body>
<article><h1>Title</h1><p>Content without any h2 headings here.</p></article>
</body></html>`;

describe("HttpAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  it("splits content at ## headings into one DocPage per section", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => null },
      text: () => Promise.resolve(MULTI_SECTION_HTML),
    }));

    const adapter = new HttpAdapter();
    const entry: LibEntry = { name: "kavach", source_type: "http", base_url: "https://kavach.dev" };
    const pages = await adapter.fetch(entry);

    expect(pages.length).toBeGreaterThanOrEqual(2);
    const titles = pages.map(p => p.title);
    expect(titles).toContain("Installation");
    expect(titles).toContain("Usage");
    pages.forEach(p => {
      expect(p.sourceType).toBe("http");
      expect(p.content).toBeTruthy();
      expect(p.description.length).toBeLessThanOrEqual(200);
    });
  });

  it("returns single DocPage when page has no ## headings", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => null },
      text: () => Promise.resolve(SINGLE_SECTION_HTML),
    }));

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "http", base_url: "https://x.com" });

    expect(pages).toHaveLength(1);
    expect(pages[0].content).toBeTruthy();
  });

  it("throws when fetch returns non-ok status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") }));
    const adapter = new HttpAdapter();
    await expect(adapter.fetch({ name: "x", source_type: "http", base_url: "https://x.com" })).rejects.toThrow("404");
  });

  it("returns raw markdown content as-is when URL ends in .md", async () => {
    const MD_BODY = `# Readme\n\n## Installation\n\nRun npm install mylib.\n\n## Usage\n\nImport and call init().`;
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/plain; charset=utf-8" },
      text: () => Promise.resolve(MD_BODY),
    }));

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "mylib", source_type: "http", base_url: "https://raw.githubusercontent.com/user/repo/main/README.md" });

    expect(pages.length).toBeGreaterThanOrEqual(1);
    // Content must not contain HTML artifacts from Readability/Turndown
    pages.forEach(p => {
      expect(p.content).not.toContain("<");
      expect(p.sourceType).toBe("http");
    });
    // Should split at ## headings
    const titles = pages.map(p => p.title);
    expect(titles).toContain("Installation");
    expect(titles).toContain("Usage");
  });

  it("returns markdown as-is when Content-Type is text/markdown", async () => {
    const MD_BODY = `## API\n\nSome API docs here.\n`;
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/markdown" },
      text: () => Promise.resolve(MD_BODY),
    }));

    const adapter = new HttpAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "http", base_url: "https://example.com/docs" });

    expect(pages[0].content).toContain("API docs here");
  });
});
