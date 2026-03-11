import { writeFile, mkdir } from "fs/promises";
import { join } from "path";
import type { ComparisonReport } from "./types.js";

function pct(n: number): string {
  return `${(n * 100).toFixed(0)}%`;
}

function bar(label: string, a: string, b: string, width = 22): string {
  return `│ ${label.padEnd(width)} │ ${a.padEnd(14)} │ ${b.padEnd(14)} │`;
}

export function printReport(report: ComparisonReport): void {
  const queryHitsCoco = report.queryResults.filter(q => q.cocoHit).length;
  const queryHitsSensei = report.queryResults.filter(q => q.senseiHit).length;

  console.log("\n┌────────────────────────┬────────────────┬────────────────┐");
  console.log("│ Metric                 │ cocoindex      │ sensei         │");
  console.log("├────────────────────────┼────────────────┼────────────────┤");
  console.log(bar("Files indexed", String(report.cocoFilesIndexed), String(report.senseiFilesIndexed)));
  console.log(bar("Coverage (vs TS files)", pct(report.cocoCoverage), pct(report.senseiCoverage)));
  console.log(bar(`Query hits (${report.queryResults.length} queries)`, String(queryHitsCoco), String(queryHitsSensei)));
  console.log("└────────────────────────┴────────────────┴────────────────┘");

  console.log("\nQuery breakdown:");
  for (const q of report.queryResults) {
    const cocoMark = q.cocoHit ? "✓" : "✗";
    const senseiMark = q.senseiHit ? "✓" : "✗";
    console.log(`  [coco:${cocoMark} sensei:${senseiMark}] ${q.query}`);
  }

  if (report.spotCheck.length > 0) {
    console.log("\n── Spot-check (rate descriptions manually 1–5) ──\n");
    for (const row of report.spotCheck) {
      const senseiDisplay = row.senseiDescription?.replace(/^\/\/\s*/, "") ?? "(not indexed)";
      console.log(`File: ${row.filePath}`);
      console.log(`  cocoindex : ${row.cocoContent ?? "(no chunk found)"}`);
      console.log(`  sensei    : ${senseiDisplay}`);
      console.log();
    }
  }
}

export async function writeMarkdownReport(
  report: ComparisonReport,
  repoPath: string
): Promise<string> {
  const date = new Date().toISOString().slice(0, 10);
  const queryHitsCoco = report.queryResults.filter(q => q.cocoHit).length;
  const queryHitsSensei = report.queryResults.filter(q => q.senseiHit).length;

  const lines = [
    `# Indexer Comparison Report — ${date}`,
    "",
    "## Summary",
    "",
    "| Metric | cocoindex | sensei |",
    "|---|---|---|",
    `| Files indexed | ${report.cocoFilesIndexed} | ${report.senseiFilesIndexed} |`,
    `| Coverage (vs TS files) | ${pct(report.cocoCoverage)} | ${pct(report.senseiCoverage)} |`,
    `| Query hits (${report.queryResults.length} queries) | ${queryHitsCoco} | ${queryHitsSensei} |`,
    "",
    "## Query Breakdown",
    "",
    ...report.queryResults.map(q =>
      `- \`${q.query}\`: coco ${q.cocoHit ? "✓" : "✗"} · sensei ${q.senseiHit ? "✓" : "✗"}`
    ),
    "",
    "## Spot-check",
    "",
    ...report.spotCheck.flatMap(row => {
      const senseiDisplay = row.senseiDescription?.replace(/^\/\/\s*/, "") ?? "(not indexed)";
      return [
        `**${row.filePath}**`,
        `- cocoindex: \`${row.cocoContent?.slice(0, 150) ?? "(no chunk)"}\``,
        `- sensei: \`${senseiDisplay}\``,
        "",
      ];
    }),
    "## Decision",
    "",
    "> Fill in after manual review.",
    "",
  ];

  const outDir = join(repoPath, "results");
  await mkdir(outDir, { recursive: true });
  const outPath = join(outDir, `indexer-comparison-${date}.md`);
  await writeFile(outPath, lines.join("\n"));
  return outPath;
}
