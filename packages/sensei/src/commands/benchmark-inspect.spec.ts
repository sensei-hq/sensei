import { describe, it, expect } from "vitest";
import { resolveBranchName } from "./benchmark-inspect.js";

describe("resolveBranchName", () => {
  it("prepends benchmark/ prefix", () => {
    expect(resolveBranchName("wild-cat-b")).toBe("benchmark/wild-cat-b");
  });

  it("does not double-prefix", () => {
    expect(resolveBranchName("benchmark/wild-cat-b")).toBe("benchmark/wild-cat-b");
  });
});
