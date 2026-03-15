// packages/engine/src/lib/llms-txt-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { writeFile, mkdtemp, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { LlmsTxtAdapter } from "./llms-txt-adapter.js";
import type { LibEntry } from "@sensei/shared";

const SAMPLE_LLMS_TXT = `# Rokkit UI
> Component library for SvelteKit

## Buttons

- [Button](https://rokkit.dev/docs/button): Primary button component with variants
- [IconButton](https://rokkit.dev/docs/icon-button): Icon-only button

## Forms

- [Input](https://rokkit.dev/docs/input): Text input field
- [Select](https://rokkit.dev/docs/select): Dropdown selector
`;

describe("LlmsTxtAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  it("fetches llms.txt from URL and parses into DocPages with correct shape", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      text: () => Promise.resolve(SAMPLE_LLMS_TXT),
    }));

    const adapter = new LlmsTxtAdapter();
    const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(4);
    expect(pages[0].title).toBe("Button");
    expect(pages[0].url).toBe("https://rokkit.dev/docs/button");
    expect(pages[0].description).toBe("Primary button component with variants");
    expect(pages[0].component).toBe("Buttons");
    expect(pages[0].content).toBeUndefined();
    expect(pages[0].sourceType).toBe("llms.txt");
    expect(pages[2].title).toBe("Input");
    expect(pages[2].component).toBe("Forms");
  });

  it("reads llms.txt from local file path", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-llmstxt-test-"));
    try {
      const localPath = join(tmpDir, "llms.txt");
      await writeFile(localPath, SAMPLE_LLMS_TXT, "utf-8");

      const adapter = new LlmsTxtAdapter();
      const entry: LibEntry = { name: "rokkit", source_type: "llms.txt", local_path: localPath };
      const pages = await adapter.fetch(entry);

      expect(pages).toHaveLength(4);
      expect(pages[0].sourceType).toBe("llms.txt");
    } finally {
      await rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("skips malformed lines gracefully", async () => {
    const malformed = `# Test\n\n## Section\n\n- [Valid](https://x.com/v): description\nnot-a-link\n- also not\n`;
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      text: () => Promise.resolve(malformed),
    }));

    const adapter = new LlmsTxtAdapter();
    const pages = await adapter.fetch({ name: "t", source_type: "llms.txt", base_url: "https://x.com/llms.txt" });

    expect(pages).toHaveLength(1);
    expect(pages[0].title).toBe("Valid");
  });

  it("throws if neither base_url nor local_path provided", async () => {
    const adapter = new LlmsTxtAdapter();
    await expect(adapter.fetch({ name: "t", source_type: "llms.txt" })).rejects.toThrow();
  });
});
