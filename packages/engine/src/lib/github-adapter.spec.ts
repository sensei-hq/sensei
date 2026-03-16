// packages/engine/src/lib/github-adapter.spec.ts
import { describe, it, expect, vi, afterEach } from "vitest";
import { GithubAdapter, parseGithubUrl } from "./github-adapter.js";
import type { LibEntry } from "@sensei/shared";

const TREE_RESPONSE = {
  tree: [
    { path: "docs/llms/README.md",   type: "blob", url: "" },
    { path: "docs/llms/api.md",      type: "blob", url: "" },
    { path: "docs/llms/Button.md",   type: "blob", url: "" },
    { path: "docs/llms/subdir",      type: "tree", url: "" },
    { path: "docs/other/skip.md",    type: "blob", url: "" },
    { path: "docs/llms/config.json", type: "blob", url: "" },
  ],
  truncated: false,
};

const README_CONTENT = `# My Library\n\nThis is the main introduction paragraph.\n\n## Section\n\nMore content.`;
const API_CONTENT    = `# API Reference\n\nDetailed API docs here.`;
const BUTTON_CONTENT = `Button component documentation.`;

describe("parseGithubUrl", () => {
  it("parses owner/repo/branch/path from a GitHub tree URL", () => {
    const result = parseGithubUrl("https://github.com/org/repo/tree/main/docs/llms");
    expect(result).toEqual({ owner: "org", repo: "repo", branch: "main", basePath: "docs/llms" });
  });

  it("handles URLs with no trailing path", () => {
    const result = parseGithubUrl("https://github.com/org/repo/tree/develop");
    expect(result).toEqual({ owner: "org", repo: "repo", branch: "develop", basePath: "" });
  });

  it("returns null for non-GitHub-tree URLs", () => {
    expect(parseGithubUrl("https://example.com/docs")).toBeNull();
    expect(parseGithubUrl("https://github.com/org/repo")).toBeNull();
  });
});

describe("GithubAdapter", () => {
  afterEach(() => vi.restoreAllMocks());

  it("fetches tree API, filters .md files under basePath, returns DocPages", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(TREE_RESPONSE) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(README_CONTENT) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(API_CONTENT) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(BUTTON_CONTENT) });
    vi.stubGlobal("fetch", fetchMock);

    const adapter = new GithubAdapter();
    const entry: LibEntry = { name: "mylib", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs/llms" };
    const pages = await adapter.fetch(entry);

    expect(pages).toHaveLength(3);
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.github.com/repos/org/repo/git/trees/main?recursive=1",
      expect.any(Object)
    );
  });

  it("sets title from first H1, summary from first paragraph, url to github blob URL", async () => {
    vi.stubGlobal("fetch", vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ tree: [{ path: "docs/readme.md", type: "blob", url: "" }], truncated: false }) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve(README_CONTENT) })
    );

    const pages = await new GithubAdapter().fetch({ name: "lib", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });

    expect(pages[0].title).toBe("My Library");
    expect(pages[0].summary).toBe("This is the main introduction paragraph.");
    expect(pages[0].url).toBe("https://github.com/org/repo/blob/main/docs/readme.md");
    expect(pages[0].sourceType).toBe("github");
    expect(pages[0].content).toBe(README_CONTENT);
  });

  it("logs a warning when tree.truncated is true", async () => {
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url.includes("api.github.com")) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({
            tree: [{ path: "docs/intro.md", type: "blob" }],
            truncated: true,
          }),
        });
      }
      return Promise.resolve({ ok: true, text: () => Promise.resolve("# Intro\n\nContent.") });
    }));

    const adapter = new GithubAdapter();
    await adapter.fetch({ name: "dbd", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });

    expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining("truncated"));
    consoleSpy.mockRestore();
  });

  it("DocPage has summary field (renamed from description)", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url.includes("api.github.com")) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ tree: [{ path: "docs/guide.md", type: "blob" }], truncated: false }) });
      }
      return Promise.resolve({ ok: true, text: () => Promise.resolve("# Guide\n\nThis is a guide.") });
    }));

    const adapter = new GithubAdapter();
    const pages = await adapter.fetch({ name: "repo", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });

    expect(pages[0].summary).toBeTruthy();
    expect(pages[0].content).toContain("This is a guide");
    expect((pages[0] as any).description).toBeUndefined();
  });

  it("infers component from immediate parent dir when nested deeper than basePath", async () => {
    vi.stubGlobal("fetch", vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ tree: [
        { path: "docs/Button/usage.md", type: "blob", url: "" },
        { path: "docs/intro.md",        type: "blob", url: "" },
      ], truncated: false }) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("Button usage docs.") })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("Introduction.") })
    );

    const pages = await new GithubAdapter().fetch({ name: "lib", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });

    expect(pages.find(p => p.title === "usage")!.component).toBe("Button");
    expect(pages.find(p => p.title === "intro")!.component).toBeUndefined();
  });

  it("falls back to filename as title when no H1 found", async () => {
    vi.stubGlobal("fetch", vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ tree: [{ path: "docs/no-heading.md", type: "blob", url: "" }], truncated: false }) })
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("Just some text, no heading.") })
    );

    const pages = await new GithubAdapter().fetch({ name: "lib", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" });
    expect(pages[0].title).toBe("no-heading");
  });

  it("throws if no base_url provided", async () => {
    await expect(new GithubAdapter().fetch({ name: "x", source_type: "github" })).rejects.toThrow("requires base_url");
  });

  it("throws if base_url is not a valid GitHub tree URL", async () => {
    await expect(new GithubAdapter().fetch({ name: "x", source_type: "github", base_url: "https://example.com/docs" })).rejects.toThrow("invalid GitHub tree URL");
  });

  it("throws if GitHub API returns an error status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 403 }));
    await expect(new GithubAdapter().fetch({ name: "x", source_type: "github", base_url: "https://github.com/org/repo/tree/main/docs" })).rejects.toThrow("GitHub API error 403");
  });
});
