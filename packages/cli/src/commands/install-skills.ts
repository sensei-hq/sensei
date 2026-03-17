import { select, multiselect, isCancel, spinner, log } from "@clack/prompts";
import { copyFile, mkdir, readdir, access } from "fs/promises";
import { join, dirname, resolve } from "path";
import { homedir } from "os";
import { fileURLToPath } from "url";
import { SKILL_CATALOG } from "../lib/skill-catalog.js";

/** Locate the bundled skills directory relative to this CLI script. */
async function findSkillsDir(): Promise<string | null> {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  const candidates = [
    join(scriptDir, "..", "skills"),            // packages/cli/skills/ (production)
    join(scriptDir, "..", "..", "..", "skills"), // monorepo root skills/ (dev)
  ];
  for (const dir of candidates) {
    try {
      await access(dir);
      return resolve(dir);
    } catch {}
  }
  return null;
}

/** Copy skills from the bundled skills dir to a target .claude/skills/ directory. */
export async function installSkillsToDir(
  skillsSourceDir: string,
  targetDir: string,
): Promise<{ installed: string[]; skipped: string[] }> {
  const entries = await readdir(skillsSourceDir, { withFileTypes: true });
  const skillDirs = entries.filter(e => e.isDirectory());

  const installed: string[] = [];
  const skipped: string[] = [];

  await mkdir(targetDir, { recursive: true });

  for (const skillDir of skillDirs) {
    const skillMd = join(skillsSourceDir, skillDir.name, "SKILL.md");
    try {
      await access(skillMd);
      const dest = join(targetDir, `${skillDir.name}.md`);
      await copyFile(skillMd, dest);
      installed.push(skillDir.name);
    } catch {
      skipped.push(skillDir.name);
    }
  }

  return { installed, skipped };
}

export type InstallScope = "repo" | "global";

/** Interactive: prompt user where to install skills, then install them. */
export async function promptAndInstallSkills(repoPath: string): Promise<void> {
  const skillsDir = await findSkillsDir();
  if (!skillsDir) {
    log.warn("Could not locate sensei skills directory — skipping skill installation.");
    return;
  }

  const entries = await readdir(skillsDir, { withFileTypes: true });
  const availableSkills = entries.filter(e => e.isDirectory()).map(e => e.name);
  if (availableSkills.length === 0) {
    log.warn("No skills found to install.");
    return;
  }

  const scope = await select({
    message: "Install sensei skills for Claude Code?",
    options: [
      { value: "repo", label: "Repo only", hint: `${repoPath}/.claude/skills/` },
      { value: "global", label: "Globally", hint: `${homedir()}/.claude/skills/` },
      { value: "skip", label: "Skip" },
    ],
  });
  if (isCancel(scope) || scope === "skip") return;

  const selected = await multiselect({
    message: "Which skills to install? (space to toggle, enter to confirm)",
    options: availableSkills.map(s => ({ value: s, label: s })),
    initialValues: availableSkills,
  });
  if (isCancel(selected) || (selected as string[]).length === 0) return;

  const targetDir = scope === "global"
    ? join(homedir(), ".claude", "skills")
    : join(repoPath, ".claude", "skills");

  const s = spinner();
  s.start("Installing skills...");

  let installedCount = 0;
  await mkdir(targetDir, { recursive: true });
  for (const skillName of selected as string[]) {
    const skillMd = join(skillsDir, skillName, "SKILL.md");
    try {
      await access(skillMd);
      await copyFile(skillMd, join(targetDir, `${skillName}.md`));
      installedCount++;
    } catch {
      // skill dir exists but no SKILL.md — skip silently
    }
  }

  s.stop(`Installed ${installedCount} skill${installedCount !== 1 ? "s" : ""} → ${targetDir}`);
}

/** Non-interactive: install all skills to the given scope. */
export async function installSkills(repoPath: string, scope: InstallScope): Promise<void> {
  const skillsDir = await findSkillsDir();
  if (!skillsDir) {
    log.warn("Could not locate sensei skills directory.");
    return;
  }

  const targetDir = scope === "global"
    ? join(homedir(), ".claude", "skills")
    : join(repoPath, ".claude", "skills");

  const s = spinner();
  s.start(`Installing skills (${scope})...`);

  const { installed } = await installSkillsToDir(skillsDir, targetDir);
  s.stop(`Installed ${installed.length} skill${installed.length !== 1 ? "s" : ""} → ${targetDir}`);
}

/** Interactive catalog-driven install — recommended skills pre-checked. */
export async function promptAndInstallSkillsFromCatalog(repoPath: string): Promise<void> {
  const skillsDir = await findSkillsDir();
  if (!skillsDir) {
    log.warn("Could not locate sensei skills directory — skipping skill installation.");
    return;
  }

  const selected = await multiselect({
    message: "Which sensei skills would you like to install? (space to toggle, enter to confirm)",
    options: SKILL_CATALOG.map(entry => ({
      value: entry.name,
      label: entry.name,
      hint: entry.description,
    })),
    initialValues: SKILL_CATALOG.filter(e => e.recommended).map(e => e.name),
    required: false,
  });
  if (isCancel(selected) || (selected as string[]).length === 0) return;

  const targetDir = join(repoPath, ".claude", "skills");
  const s = spinner();
  s.start("Installing skills...");

  let installedCount = 0;
  await mkdir(targetDir, { recursive: true });
  for (const skillName of selected as string[]) {
    const skillMd = join(skillsDir, skillName, "SKILL.md");
    try {
      await access(skillMd);
      await copyFile(skillMd, join(targetDir, `${skillName}.md`));
      installedCount++;
    } catch {
      // skill not present in this build — skip
    }
  }
  s.stop(`Installed ${installedCount} skill${installedCount !== 1 ? "s" : ""} → ${targetDir}`);
}

/** Non-interactive: install skills from the catalog filtered by mode. */
export async function installSkillsFromCatalog(
  repoPath: string,
  mode: "recommended" | "all",
): Promise<void> {
  const skillsDir = await findSkillsDir();
  if (!skillsDir) {
    log.warn("Could not locate sensei skills directory.");
    return;
  }

  const entries = mode === "recommended"
    ? SKILL_CATALOG.filter(e => e.recommended)
    : SKILL_CATALOG;

  const targetDir = join(repoPath, ".claude", "skills");
  const s = spinner();
  s.start(`Installing ${mode} skills...`);

  let installedCount = 0;
  await mkdir(targetDir, { recursive: true });
  for (const entry of entries) {
    const skillMd = join(skillsDir, entry.name, "SKILL.md");
    try {
      await access(skillMd);
      await copyFile(skillMd, join(targetDir, `${entry.name}.md`));
      installedCount++;
    } catch {
      // skill not present in this build — skip
    }
  }
  s.stop(`Installed ${installedCount} skill${installedCount !== 1 ? "s" : ""} → ${targetDir}`);
}
