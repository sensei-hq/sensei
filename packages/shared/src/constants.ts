import { join } from "path";

/** The folder where sensei writes all generated artifacts. */
export const SENSEI_DIR = ".sensei";

/** Build a path inside the sensei dir relative to repoPath. */
export function senseiPath(repoPath: string, ...parts: string[]): string {
  return join(repoPath, SENSEI_DIR, ...parts);
}
