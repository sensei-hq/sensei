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
    const entry: LibEntry = { name: "mylib", source_type: "local", local_path: tmpDir };
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
    const pages = await adapter.fetch({ name: "lib", source_type: "local", local_path: tmpDir });

    const btn = pages.find(p => p.title === "button")!;
    expect(btn.component).toBe("Button");

    const intro = pages.find(p => p.title === "intro")!;
    expect(intro.component).toBeUndefined();
  });

  it("throws if local_path not provided", async () => {
    const adapter = new LocalAdapter();
    await expect(adapter.fetch({ name: "x", source_type: "local" })).rejects.toThrow();
  });
});
