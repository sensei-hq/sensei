import { describe, it, expect, afterEach } from "vitest";
import { mkdtemp, rm, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { LocalAdapter } from "./local-adapter.js";
import type { LibEntry } from "@sensei/shared";

describe("LocalAdapter", () => {
  let tmpDir: string;
  afterEach(async () => { if (tmpDir) await rm(tmpDir, { recursive: true, force: true }); });

  it("collects .md and .txt files, sets title from filename and summary from extractSummary", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-local-test-"));
    await writeFile(join(tmpDir, "overview.md"), "# Overview\n\nThis is the overview.", "utf-8");
    await writeFile(join(tmpDir, "api.txt"), "API reference content here.", "utf-8");
    await writeFile(join(tmpDir, "ignored.json"), '{"skip":true}', "utf-8");

    const adapter = new LocalAdapter();
    const entry: LibEntry = { name: "mylib", source_type: "local", base_url: `file://${tmpDir}` };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(2);
    const titles = pages.map(p => p.title).sort();
    expect(titles).toEqual(["api", "overview"]);

    const overview = pages.find(p => p.title === "overview")!;
    expect(overview.content).toContain("# Overview");
    expect(overview.summary).toBeTruthy();
    expect(overview.localPath).toContain("overview.md");
    expect(overview.url).toBeUndefined();
    expect(overview.component).toBeUndefined();
    expect(overview.sourceType).toBe("local");
  });

  it("infers component from immediate parent dir name when file is nested", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-local-test-"));
    const btnDir = join(tmpDir, "Button");
    await mkdir(btnDir, { recursive: true });
    await writeFile(join(btnDir, "button.md"), "Button component docs.", "utf-8");
    await writeFile(join(tmpDir, "intro.md"), "Introduction.", "utf-8");

    const adapter = new LocalAdapter();
    const pages = await adapter.fetch({ name: "lib", source_type: "local", base_url: `file://${tmpDir}` });

    const btn = pages.find(p => p.title === "button")!;
    expect(btn.component).toBe("Button");

    const intro = pages.find(p => p.title === "intro")!;
    expect(intro.component).toBeUndefined();
  });

  it("uses llms.txt index when present — returns curated titles, summaries, and components", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-local-test-"));
    const llmsTxt = `# MyLib\n\n## Core\n\n- [Auth](./auth.txt): Core auth module\n- [UI](./ui.txt): UI components\n`;
    await writeFile(join(tmpDir, "llms.txt"), llmsTxt, "utf-8");
    await writeFile(join(tmpDir, "auth.txt"), "# Auth\n\nFull auth docs.", "utf-8");
    await writeFile(join(tmpDir, "ui.txt"), "# UI\n\nFull UI docs.", "utf-8");
    // extra file that should NOT appear (no llms.txt entry)
    await writeFile(join(tmpDir, "internal.txt"), "Internal notes.", "utf-8");

    const adapter = new LocalAdapter();
    const pages = await adapter.fetch({ name: "mylib", source_type: "local", base_url: `file://${tmpDir}` });

    expect(pages).toHaveLength(2);
    expect(pages.map(p => p.title)).toEqual(["Auth", "UI"]);
    expect(pages[0].summary).toBe("Core auth module");
    expect(pages[0].component).toBe("Core");
    expect(pages[0].content).toContain("Full auth docs");
    expect(pages[0].localPath).toContain("auth.txt");
    expect(pages[0].url).toBeUndefined();
    expect(pages[0].sourceType).toBe("local");
    expect(pages[0].sequence).toBe(0);
  });

  it("falls back to walkDir when no llms.txt present", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-local-test-"));
    await writeFile(join(tmpDir, "doc.md"), "# Doc\n\nContent.", "utf-8");

    const pages = await new LocalAdapter().fetch({ name: "lib", source_type: "local", base_url: `file://${tmpDir}` });
    expect(pages).toHaveLength(1);
    expect(pages[0].title).toBe("doc");
  });

  it("throws if base_url not provided", async () => {
    const adapter = new LocalAdapter();
    // @ts-expect-error — testing runtime guard
    await expect(adapter.fetch({ name: "x", source_type: "local" })).rejects.toThrow("requires a file:// base_url");
  });
});
