import fg from "fast-glob";
import { readFile } from "fs/promises";

const EXPORT_RE = /^\s*export\s+(async\s+)?(function|class|const|let|var|type|interface|enum)\s+(\w+)/gm;

export interface GroundTruth {
  files: string[];
  exportCount: number;
}

export async function extractGroundTruth(repoPath: string): Promise<GroundTruth> {
  const tsFiles = await fg(["packages/*/src/**/*.ts", "apps/*/src/**/*.ts"], {
    cwd: repoPath,
    ignore: ["**/*.spec.ts", "**/*.test.ts", "**/node_modules/**", "**/*.d.ts"],
    absolute: false,
  });

  let exportCount = 0;
  for (const file of tsFiles) {
    const content = await readFile(`${repoPath}/${file}`, "utf-8");
    const matches = content.match(EXPORT_RE);
    if (matches) exportCount += matches.length;
  }

  return { files: tsFiles, exportCount };
}
