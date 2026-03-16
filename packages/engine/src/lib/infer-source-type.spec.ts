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

  it("detects local path", () => {
    const r = inferSourceType("/Users/jerry/projects/mylib/docs");
    expect(r.source_type).toBe("local");
    expect((r as any).local_path).toBe("/Users/jerry/projects/mylib/docs");
  });

  it("falls through to http (not github) for non-tree github URLs", () => {
    const r = inferSourceType("https://github.com/org/repo");
    expect(r.source_type).toBe("http");
  });

  it("falls through to http on malformed URL that starts with https://", () => {
    const r = inferSourceType("https://");
    expect(r.source_type).toBe("http");
  });
});
