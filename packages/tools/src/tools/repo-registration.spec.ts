import { describe, it, expect, vi } from "vitest";
import { registerRepo } from "./repo-registration.js";

const mockSelect = vi.fn(() => ({ data: [{ id: "new-repo-uuid" }], error: null }));
const mockUpsert = vi.fn(() => ({ select: mockSelect }));
const mockFrom = vi.fn(() => ({ upsert: mockUpsert }));
const mockClient = { from: mockFrom } as any;

describe("registerRepo", () => {
  it("upserts repo and returns repo_id", async () => {
    const repoId = await registerRepo(mockClient, {
      name: "sensei",
      remote_url: "git@github.com:org/sensei.git",
      default_branch: "main",
    });
    expect(repoId).toBe("new-repo-uuid");
  });

  it("returns null when upsert errors", async () => {
    const errClient = {
      from: vi.fn(() => ({
        upsert: vi.fn(() => ({
          select: vi.fn(() => ({ data: null, error: new Error("fail") }))
        }))
      }))
    } as any;
    const repoId = await registerRepo(errClient, { name: "x", remote_url: null });
    expect(repoId).toBeNull();
  });
});
