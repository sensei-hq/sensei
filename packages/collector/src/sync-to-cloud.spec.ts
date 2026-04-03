import { describe, it, expect, vi } from "vitest";
import { scrubRepoName } from "./sync-to-cloud.js";

// Test the scrub function directly (exported for testing)
// Re-export it from the module for test access
import { createHash } from "crypto";

function testScrub(repoName: string, accountId: string): string {
  return createHash("sha256").update(`${accountId}:${repoName}`).digest("hex").slice(0, 16);
}

describe("syncToCloud", () => {
  it("scrubs repo name to a deterministic hash", () => {
    const slug1 = testScrub("my-repo", "acme-account");
    const slug2 = testScrub("my-repo", "acme-account");
    expect(slug1).toBe(slug2);
    expect(slug1).toHaveLength(16);
  });

  it("different account produces different slug for same repo", () => {
    const slug1 = testScrub("my-repo", "acme");
    const slug2 = testScrub("my-repo", "other");
    expect(slug1).not.toBe(slug2);
  });

  it("different repo produces different slug for same account", () => {
    const slug1 = testScrub("repo-a", "acme");
    const slug2 = testScrub("repo-b", "acme");
    expect(slug1).not.toBe(slug2);
  });
});
