// packages/engine/src/lib/doc-utils.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { resolveUrl, fetchAsMarkdown, extractSummary, splitSections } from "./doc-utils.js";

describe("resolveUrl", () => {
  it("resolves relative URL against base", () => {
    expect(resolveUrl("https://example.com/docs/", "../api/intro")).toBe("https://example.com/api/intro");
  });

  it("returns absolute URL unchanged", () => {
    expect(resolveUrl("https://example.com/docs/", "https://other.com/page")).toBe("https://other.com/page");
  });

  it("resolves root-relative path", () => {
    expect(resolveUrl("https://example.com/docs/guide", "/api/ref")).toBe("https://example.com/api/ref");
  });
});

describe("fetchAsMarkdown", () => {
  afterEach(() => vi.restoreAllMocks());

  it("returns body as-is when URL ends in .md", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/html" },
      text: () => Promise.resolve("# Markdown"),
    }));
    const result = await fetchAsMarkdown("https://example.com/docs/README.md");
    expect(result).toBe("# Markdown");
  });

  it("returns body as-is when content-type is text/plain", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/plain" },
      text: () => Promise.resolve("# Plain text"),
    }));
    const result = await fetchAsMarkdown("https://example.com/llms.txt");
    expect(result).toBe("# Plain text");
  });

  it("converts HTML to markdown via Readability+Turndown for text/html", async () => {
    const html = `<html><body><article><h1>Title</h1><p>Paragraph text.</p></article></body></html>`;
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => "text/html" },
      text: () => Promise.resolve(html),
    }));
    const result = await fetchAsMarkdown("https://example.com/page");
    expect(result).toContain("Paragraph text");
    expect(result).not.toContain("<p>");
  });

  it("throws on HTTP error status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, status: 404,
      headers: { get: () => null },
      text: () => Promise.resolve(""),
    }));
    await expect(fetchAsMarkdown("https://example.com/missing")).rejects.toThrow("404");
  });
});

describe("extractSummary", () => {
  it("returns first non-heading paragraph trimmed to ≤200 chars", () => {
    const md = `# Title\n\nThis is the first paragraph with content.\n\n## Section\n\nMore content.`;
    expect(extractSummary(md)).toBe("This is the first paragraph with content.");
  });

  it("joins multi-line paragraph into single string", () => {
    const md = `# Title\n\nLine one.\nLine two.\nLine three.`;
    expect(extractSummary(md)).toBe("Line one. Line two. Line three.");
  });

  it("skips heading lines", () => {
    const md = `## Heading\n\nActual content here.`;
    expect(extractSummary(md)).toBe("Actual content here.");
  });

  it("truncates to ≤200 chars", () => {
    const long = "A".repeat(300);
    const md = `# Title\n\n${long}`;
    const result = extractSummary(md);
    expect(result.length).toBeLessThanOrEqual(200);
  });

  it("returns empty string when no non-heading paragraph", () => {
    const md = `# Just a heading`;
    expect(extractSummary(md)).toBe("");
  });
});

describe("splitSections", () => {
  const MD = `# Title\n\nIntro paragraph.\n\n## Installation\n\nRun npm install.\n\n## Usage\n\nCall init().`;

  it("splits at H2 headings", () => {
    const sections = splitSections(MD);
    const titles = sections.map(s => s.title);
    expect(titles).toContain("Installation");
    expect(titles).toContain("Usage");
  });

  it("includes Overview section for pre-H2 content", () => {
    const sections = splitSections(MD);
    expect(sections[0].title).toBe("Overview");
    expect(sections[0].content).toContain("Intro paragraph");
    expect(sections[0].sequence).toBe(0);
  });

  it("omits Overview when pre-H2 content is empty/whitespace", () => {
    const md = `## Installation\n\nRun npm install.`;
    const sections = splitSections(md);
    expect(sections[0].title).toBe("Installation");
  });

  it("assigns sequential sequence numbers", () => {
    const sections = splitSections(MD);
    sections.forEach((s, i) => expect(s.sequence).toBe(i));
  });

  it("omits sections with empty content after whitespace trim", () => {
    const md = `## Empty\n\n   \n\n## Real\n\nActual content.`;
    const sections = splitSections(md);
    expect(sections.map(s => s.title)).not.toContain("Empty");
    expect(sections.map(s => s.title)).toContain("Real");
  });

  it("includes H3+ content within the enclosing H2 section", () => {
    const md = `## Usage\n\n### Sub\n\nSub content.`;
    const sections = splitSections(md);
    expect(sections[0].content).toContain("Sub content");
  });

  it("returns empty array for empty input", () => {
    expect(splitSections("")).toHaveLength(0);
    expect(splitSections("   ")).toHaveLength(0);
  });
});
