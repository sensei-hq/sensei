// packages/engine/src/lib/infer-source-type.spec.ts
import { describe, it, expect } from "vitest";
import { inferSourceType } from "./infer-source-type.js";

describe("inferSourceType", () => {
  it("detects llms.txt URL", () => {
    const r = inferSourceType("https://kavach.dev/llms.txt");
    expect(r.source_type).toBe("llms.txt");
    expect(r.base_url).toBe("https://kavach.dev/llms.txt");
  });

  it("detects GitHub tree URL", () => {
    const r = inferSourceType("https://github.com/org/repo/tree/main/docs");
    expect(r.source_type).toBe("github");
    expect(r.base_url).toBe("https://github.com/org/repo/tree/main/docs");
  });

  it("detects plain HTTP URL as http", () => {
    const r = inferSourceType("https://docs.example.com/api");
    expect(r.source_type).toBe("http");
    expect(r.base_url).toBe("https://docs.example.com/api");
  });

  it("converts absolute path to file:// URL with source_type local", () => {
    const r = inferSourceType("/Users/jerry/projects/mylib/docs");
    expect(r.source_type).toBe("local");
    expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/docs");
    expect((r as any).local_path).toBeUndefined();
  });

  it("converts absolute .txt path to file:// URL with source_type llms.txt", () => {
    const r = inferSourceType("/Users/jerry/projects/mylib/docs/llms.txt");
    expect(r.source_type).toBe("llms.txt");
    expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/docs/llms.txt");
  });

  it("passes through file:// URL as local source type", () => {
    const r = inferSourceType("file:///Users/jerry/projects/mylib/docs");
    expect(r.source_type).toBe("local");
    expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/docs");
  });

  it("passes through file:// .txt URL as llms.txt source type", () => {
    const r = inferSourceType("file:///Users/jerry/projects/mylib/llms.txt");
    expect(r.source_type).toBe("llms.txt");
    expect(r.base_url).toBe("file:///Users/jerry/projects/mylib/llms.txt");
  });

  it("falls through to http (not github) for non-tree github URLs", () => {
    const r = inferSourceType("https://github.com/org/repo");
    expect(r.source_type).toBe("http");
  });

  it("falls through to http on malformed URL that starts with https://", () => {
    const r = inferSourceType("https://");
    expect(r.source_type).toBe("http");
  });

  it("detects .txt URLs (not ending in /llms.txt) as llms.txt type", () => {
    const r = inferSourceType("https://rokkit.vercel.app/llms/index.txt");
    expect(r.source_type).toBe("llms.txt");
    expect(r.base_url).toBe("https://rokkit.vercel.app/llms/index.txt");
  });

  it("detects any .txt URL as llms.txt, not just /llms.txt suffix", () => {
    const r = inferSourceType("https://example.com/components.txt");
    expect(r.source_type).toBe("llms.txt");
  });
});
