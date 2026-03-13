import { describe, it, expect } from "vitest";
import { MarkdownAdapter } from "./markdown.js";

describe("MarkdownAdapter", () => {
  const adapter = new MarkdownAdapter();

  it("splits H2 sections", () => {
    const content = ["## Overview", "This is overview.", "", "## Usage", "Use it."].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections).toHaveLength(2);
    expect(sections[0].heading).toBe("Overview");
    expect(sections[0].level).toBe(2);
    expect(sections[0].content).toContain("overview");
    expect(sections[1].heading).toBe("Usage");
  });

  it("splits H3 sections", () => {
    const content = ["## Parent", "intro", "### Sub Section", "sub content"].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections.some(s => s.level === 3 && s.heading === "Sub Section")).toBe(true);
  });

  it("records startLine and endLine (1-indexed)", () => {
    const content = ["## First", "content", "## Second", "more"].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections[0].startLine).toBe(1);
    expect(sections[0].endLine).toBe(2);
    expect(sections[1].startLine).toBe(3);
    expect(sections[1].endLine).toBe(4);
  });

  it("extracts codeRefs from backtick identifiers", () => {
    const content = ["## Overview", "Call `createClient()` to start. Use `makeSenseiClient`."].join("\n");
    const sections = adapter.parse("docs/guide.md", content);
    expect(sections[0].codeRefs).toContain("createClient");
    expect(sections[0].codeRefs).toContain("makeSenseiClient");
  });

  it("returns [] for content with no H2/H3 headings", () => {
    expect(adapter.parse("docs/guide.md", "# Title\n\nJust a paragraph.\n")).toHaveLength(0);
  });

  it("extensions includes .md and .mdx", () => {
    expect(adapter.extensions).toContain(".md");
    expect(adapter.extensions).toContain(".mdx");
  });
});
