import { vi, describe, it, expect, afterEach } from "vitest";
vi.unmock("fs/promises");
import { mkdtemp, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { ClaudeAdapter } from "./claude-adapter.js";

describe("ClaudeAdapter", () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
  });

  it("writeSkills creates correct filenames and returns AgentSkillFile[]", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-adapter-test-"));
    const adapter = new ClaudeAdapter(tmpDir); // inject skillsDir for testing

    const skills = {
      orientation: "---\nname: test\ndescription: test\n---\n# Orientation",
      workflow: "---\nname: wf\ndescription: wf\n---\n# Workflow",
      context: "---\nname: ctx\ndescription: ctx\n---\n# Context",
      patterns: "---\nname: pat\ndescription: pat\n---\n# Patterns",
    };

    const result = await adapter.writeSkills(skills, "my-repo");

    expect(result).toHaveLength(4);
    expect(result.map(f => f.category).sort()).toEqual(["context", "orientation", "patterns", "workflow"]);
    expect(result[0].path).toContain("sensei-my-repo-");
    expect(result[0].generatedAt).toBeTruthy();
  });

  it("installedSkills returns only files matching the repo slug prefix", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-adapter-test-"));
    const adapter = new ClaudeAdapter(tmpDir);

    // Write skills for two different repos
    await adapter.writeSkills({ orientation: "# A" }, "repo-a");
    await adapter.writeSkills({ orientation: "# B" }, "repo-b");

    const repoASkills = await adapter.installedSkills("repo-a");
    expect(repoASkills).toHaveLength(1);
    expect(repoASkills[0].category).toBe("orientation");

    const repoBSkills = await adapter.installedSkills("repo-b");
    expect(repoBSkills).toHaveLength(1);
  });

  it("installedSkills returns empty array when directory does not exist", async () => {
    const adapter = new ClaudeAdapter("/nonexistent/path/skills");
    const result = await adapter.installedSkills("any-repo");
    expect(result).toEqual([]);
  });
});
