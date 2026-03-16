// packages/engine/src/lib/llms-txt-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { mkdtemp, rm, writeFile } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { LlmsTxtAdapter } from "./llms-txt-adapter.js";
import type { LibEntry } from "@sensei/shared";

const INDEX = `# Rokkit UI
> Component library

## Buttons

- [Button](https://rokkit.dev/docs/button): Primary button component
- [IconButton](https://rokkit.dev/docs/icon-button): Icon-only button

## Forms

- [Input](https://rokkit.dev/docs/input): Text input field
`;

const BUTTON_MD = `# Button\n\n## Overview\n\nFull button docs.\n\n## Props\n\nVariants, size.`;
const ICON_MD = `# IconButton\n\nIcon-only button docs.`;
const INPUT_MD = `# Input\n\nFull input docs.`;

describe("LlmsTxtAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  function mockFetch(responses: Record<string, string>) {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url in responses) {
        return Promise.resolve({
          ok: true,
          // headers needed by fetchAsMarkdown to check content-type
          headers: { get: () => "text/plain" },
          text: () => Promise.resolve(responses[url]),
        });
      }
      return Promise.resolve({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));
  }

  it("fetches index and then each linked page, returning one DocPage per link", async () => {
    mockFetch({
      "https://rokkit.dev/llms/index.txt": INDEX,
      "https://rokkit.dev/docs/button": BUTTON_MD,
      "https://rokkit.dev/docs/icon-button": ICON_MD,
      "https://rokkit.dev/docs/input": INPUT_MD,
    });

    const adapter = new LlmsTxtAdapter();
    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms/index.txt" };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(3);
    expect(pages[0].title).toBe("Button");
    expect(pages[0].url).toBe("https://rokkit.dev/docs/button");
    // summary = index description (authoritative)
    expect(pages[0].summary).toBe("Primary button component");
    // content = fetched markdown from the linked URL
    expect(pages[0].content).toContain("Full button docs");
    expect(pages[0].component).toBe("Buttons");
    expect(pages[0].sequence).toBe(0);
    expect(pages[0].sourceType).toBe("llms.txt");
  });

  it("resolves relative URLs in the index against the index URL", async () => {
    const indexWithRelative = `- [Page](./page): A page`;
    mockFetch({
      "https://rokkit.dev/docs/llms.txt": indexWithRelative,
      "https://rokkit.dev/docs/page": "# Page\n\nContent.",
    });

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "r", source_type: "llms.txt", base_url: "https://rokkit.dev/docs/llms.txt" });

    expect(pages[0].url).toBe("https://rokkit.dev/docs/page");
    expect(pages[0].content).toContain("Content");
  });

  it("gracefully degrades when a linked page fetch fails — uses summary as content", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url === "https://rokkit.dev/llms.txt") {
        return Promise.resolve({ ok: true, headers: { get: () => "text/plain" }, text: () => Promise.resolve(`- [Btn](https://rokkit.dev/btn): Button desc`) });
      }
      return Promise.resolve({ ok: false, status: 500, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "r", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" });

    expect(pages).toHaveLength(1);
    // Falls back to summary when fetch fails
    expect(pages[0].content).toBe("Button desc");
    expect(pages[0].summary).toBe("Button desc");
  });

  it("skips malformed lines gracefully", async () => {
    const malformed = `## Section\n\n- [Valid](https://x.com/v): desc\nnot-a-link\n`;
    // Use mockFetch which adds headers to all responses
    mockFetch({
      "https://x.com/llms.txt": malformed,
      "https://x.com/v": "# Valid\n\nContent.",
    });

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "t", source_type: "llms.txt", base_url: "https://x.com/llms.txt" });

    expect(pages).toHaveLength(1);
    expect(pages[0].title).toBe("Valid");
  });

  it("reads local files when base_url is a file:// URL", async () => {
    let tmpDir!: string;
    try {
      tmpDir = await mkdtemp(join(tmpdir(), "sensei-llms-test-"));
      const llmsTxt = `## Core\n\n- [Auth](./auth.txt): Core auth module\n- [UI](./ui.txt): UI components\n`;
      await writeFile(join(tmpDir, "llms.txt"), llmsTxt, "utf-8");
      await writeFile(join(tmpDir, "auth.txt"), "# Auth\n\nFull auth docs.", "utf-8");
      await writeFile(join(tmpDir, "ui.txt"), "# UI\n\nFull UI docs.", "utf-8");

      const pages = await new LlmsTxtAdapter().fetch({
        name: "lib",
        source_type: "llms.txt",
        base_url: `file://${join(tmpDir, "llms.txt")}`,
      });

      expect(pages).toHaveLength(2);
      expect(pages.map(p => p.title)).toEqual(["Auth", "UI"]);
      expect(pages[0].summary).toBe("Core auth module");
      expect(pages[0].component).toBe("Core");
      expect(pages[0].content).toContain("Full auth docs");
      expect(pages[0].url).toBeUndefined();
      expect(pages[0].localPath).toContain("auth.txt");
      expect(pages[0].sourceType).toBe("llms.txt");
    } finally {
      if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("throws if base_url not provided", async () => {
    // @ts-expect-error — testing runtime guard
    await expect(new LlmsTxtAdapter().fetch({ name: "t", source_type: "llms.txt" })).rejects.toThrow();
  });
});
