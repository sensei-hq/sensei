// packages/engine/src/lib/adapter-consistency.spec.ts
/**
 * Cross-adapter consistency tests.
 *
 * Given the same llms.txt index content and linked documents, all three adapters
 * (LlmsTxtAdapter, LocalAdapter, GithubAdapter) must produce the same set of
 * document titles and component groupings, regardless of how the content is
 * sourced (HTTP, local file system, or GitHub tree API).
 *
 * This mirrors the real-world requirement that:
 *   /path/to/docs/llms  (LocalAdapter)
 *   https://github.com/.../docs/llms  (GithubAdapter)
 *   https://example.com/llms/llms.txt (LlmsTxtAdapter via HTTP)
 *   /path/to/docs/llms/llms.txt       (LlmsTxtAdapter via file:// URL)
 * all produce the same document set.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { mkdtemp, rm, writeFile } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { LlmsTxtAdapter } from "./llms-txt-adapter.js";
import { LocalAdapter } from "./local-adapter.js";
import { GithubAdapter } from "./github-adapter.js";

// Shared fixture: a minimal llms.txt with two sections and three docs
const LLMS_TXT = `# MyLib

## Core

- [Auth](./auth.txt): Core auth module
- [UI](./ui.txt): UI components

## Adapters

- [Supabase](./adapter-supabase.txt): Supabase adapter
`;

const AUTH_CONTENT = "# Auth\n\nFull auth documentation.";
const UI_CONTENT = "# UI\n\nFull UI documentation.";
const SUPABASE_CONTENT = "# Supabase\n\nFull Supabase adapter documentation.";

const EXPECTED_TITLES = ["Auth", "UI", "Supabase"];
const EXPECTED_COMPONENTS = ["Core", "Core", "Adapters"];
const EXPECTED_SUMMARIES = ["Core auth module", "UI components", "Supabase adapter"];

describe("adapter consistency — same llms.txt content → same docs across all adapters", () => {
  afterEach(() => vi.restoreAllMocks());

  it("LlmsTxtAdapter (HTTP) produces correct titles, components, summaries", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url.endsWith("llms.txt")) return Promise.resolve({ ok: true, headers: { get: () => "text/plain" }, text: () => Promise.resolve(LLMS_TXT) });
      if (url.endsWith("auth.txt")) return Promise.resolve({ ok: true, headers: { get: () => "text/plain" }, text: () => Promise.resolve(AUTH_CONTENT) });
      if (url.endsWith("ui.txt")) return Promise.resolve({ ok: true, headers: { get: () => "text/plain" }, text: () => Promise.resolve(UI_CONTENT) });
      if (url.endsWith("adapter-supabase.txt")) return Promise.resolve({ ok: true, headers: { get: () => "text/plain" }, text: () => Promise.resolve(SUPABASE_CONTENT) });
      return Promise.resolve({ ok: false, status: 404, headers: { get: () => null }, text: () => Promise.resolve("") });
    }));

    const pages = await new LlmsTxtAdapter().fetch({ name: "mylib", source_type: "llms.txt", base_url: "https://example.com/llms/llms.txt" });

    expect(pages.map(p => p.title)).toEqual(EXPECTED_TITLES);
    expect(pages.map(p => p.component)).toEqual(EXPECTED_COMPONENTS);
    expect(pages.map(p => p.summary)).toEqual(EXPECTED_SUMMARIES);
    expect(pages.every(p => p.sourceType === "llms.txt")).toBe(true);
  });

  it("LlmsTxtAdapter (file:// URL) produces same titles, components, summaries", async () => {
    let tmpDir!: string;
    try {
      tmpDir = await mkdtemp(join(tmpdir(), "sensei-consistency-test-"));
      await writeFile(join(tmpDir, "llms.txt"), LLMS_TXT, "utf-8");
      await writeFile(join(tmpDir, "auth.txt"), AUTH_CONTENT, "utf-8");
      await writeFile(join(tmpDir, "ui.txt"), UI_CONTENT, "utf-8");
      await writeFile(join(tmpDir, "adapter-supabase.txt"), SUPABASE_CONTENT, "utf-8");

      const pages = await new LlmsTxtAdapter().fetch({ name: "mylib", source_type: "llms.txt", base_url: `file://${join(tmpDir, "llms.txt")}` });

      expect(pages.map(p => p.title)).toEqual(EXPECTED_TITLES);
      expect(pages.map(p => p.component)).toEqual(EXPECTED_COMPONENTS);
      expect(pages.map(p => p.summary)).toEqual(EXPECTED_SUMMARIES);
      expect(pages.every(p => p.sourceType === "llms.txt")).toBe(true);
    } finally {
      if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("LocalAdapter (with llms.txt) produces same titles, components, summaries", async () => {
    let tmpDir!: string;
    try {
      tmpDir = await mkdtemp(join(tmpdir(), "sensei-consistency-test-"));
      await writeFile(join(tmpDir, "llms.txt"), LLMS_TXT, "utf-8");
      await writeFile(join(tmpDir, "auth.txt"), AUTH_CONTENT, "utf-8");
      await writeFile(join(tmpDir, "ui.txt"), UI_CONTENT, "utf-8");
      await writeFile(join(tmpDir, "adapter-supabase.txt"), SUPABASE_CONTENT, "utf-8");

      const pages = await new LocalAdapter().fetch({ name: "mylib", source_type: "local", base_url: `file://${tmpDir}` });

      expect(pages.map(p => p.title)).toEqual(EXPECTED_TITLES);
      expect(pages.map(p => p.component)).toEqual(EXPECTED_COMPONENTS);
      expect(pages.map(p => p.summary)).toEqual(EXPECTED_SUMMARIES);
      expect(pages.every(p => p.sourceType === "local")).toBe(true);
    } finally {
      if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
    }
  });

  it("GithubAdapter (with llms.txt in tree) produces same titles, components, summaries", async () => {
    const treeWithIndex = {
      tree: [
        { path: "docs/llms/llms.txt",           type: "blob", url: "" },
        { path: "docs/llms/auth.txt",            type: "blob", url: "" },
        { path: "docs/llms/ui.txt",              type: "blob", url: "" },
        { path: "docs/llms/adapter-supabase.txt", type: "blob", url: "" },
      ],
      truncated: false,
    };
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url.includes("api.github.com")) return Promise.resolve({ ok: true, json: () => Promise.resolve(treeWithIndex) });
      if (url.includes("raw.githubusercontent.com")) {
        if (url.endsWith("llms.txt")) return Promise.resolve({ ok: true, text: () => Promise.resolve(LLMS_TXT) });
        if (url.endsWith("auth.txt")) return Promise.resolve({ ok: true, text: () => Promise.resolve(AUTH_CONTENT) });
        if (url.endsWith("ui.txt")) return Promise.resolve({ ok: true, text: () => Promise.resolve(UI_CONTENT) });
        if (url.endsWith("adapter-supabase.txt")) return Promise.resolve({ ok: true, text: () => Promise.resolve(SUPABASE_CONTENT) });
      }
      return Promise.resolve({ ok: false, status: 404 });
    }));

    const pages = await new GithubAdapter().fetch({ name: "mylib", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs/llms" });

    expect(pages.map(p => p.title)).toEqual(EXPECTED_TITLES);
    expect(pages.map(p => p.component)).toEqual(EXPECTED_COMPONENTS);
    expect(pages.map(p => p.summary)).toEqual(EXPECTED_SUMMARIES);
    expect(pages.every(p => p.sourceType === "github")).toBe(true);
  });
});
