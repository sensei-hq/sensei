#!/usr/bin/env node
import { parseArgs } from "node:util";

const { positionals, values } = parseArgs({
  args: process.argv.slice(2),
  allowPositionals: true,
  options: {
    help: { type: "boolean", short: "h", default: false },
    force: { type: "boolean", default: false },
    "fail-on-drift": { type: "boolean", default: false },
    "dry-run": { type: "boolean", default: false },
    template: { type: "string" },
    examples: { type: "string" },
    sample: { type: "string" },   // number parsed manually
    port: { type: "string" },
    db: { type: "string" },
    // benchmark doctor named params
    source: { type: "string" },
    dest: { type: "string" },
    verbose: { type: "boolean", default: false },
  },
});

const [cmd, ...rest] = positionals;

// Always operate from repo root — never scatter .index folders across the repo
import { findRepoRoot } from "./git.js";
import { existsSync } from "node:fs";
import { join as pathJoin } from "node:path";
const _cwd = process.cwd();
const repoRoot = findRepoRoot(_cwd);

// Guard: bail if we can't find a real repo root
if (repoRoot === _cwd && !existsSync(pathJoin(_cwd, ".git")) && !existsSync(pathJoin(_cwd, "package.json"))) {
  console.error("sensei: could not detect repo root. Run sensei from inside a git repo or a directory with package.json.");
  process.exit(1);
}
// Guard: bail if running from inside .index/
if (_cwd.includes("/.index") || _cwd.includes("\\.index")) {
  console.error(`sensei: do not run sensei from inside .index/. Change to the repo root and try again.`);
  process.exit(1);
}
if (repoRoot !== _cwd) {
  console.error(`sensei: using repo root ${repoRoot}`);
}

const HELP = `sensei — AI skills toolchain

Usage:
  sensei <command> [options]

Commands:
  init                     Set up a new repo (index + profiles + hook)
  add                      Add sensei to an existing repo (non-destructive)
  status                   Show index age, drift status, active profiles
  index                    Re-index the current repo
  drift                    Check for doc drift
  doctor <path>            Reformat docs to match canonical templates
  migrate                  Migrate agents/ folder to .index/checkpoints/
  benchmark doctor         Run 3-strategy doc conversion benchmark
  benchmark inspect        Switch to a benchmark branch for inspection
  benchmark promote        Merge chosen strategy branch, submit telemetry
  serve                    Start local telemetry report receiver

Options:
  -h, --help               Show this help message

index:
  --force                  Force full re-scan (ignore cached fingerprints)

drift:
  --fail-on-drift          Exit 1 if any drift is detected (for CI)

doctor:
  --dry-run                Show what would change without writing files
  --template <path>        Path to template file (default: docs/templates/feature.md)

benchmark doctor:
  --source <dir>           Input directory (e.g. docs/requirements)
  --dest <dir>             Output directory (e.g. docs/features)
  --template <path>        Template file (default: docs/templates/feature.md)
  --examples <dir>         Examples directory — can be absolute path (e.g. /path/to/other-repo/docs/features)
  --sample <n>             Process only the first N input files
  --verbose                Show git steps, prompt sizes, and Claude API call markers

benchmark inspect:
  sensei benchmark inspect <run>-<a|b|c>

benchmark promote:
  sensei benchmark promote <run-name>

serve:
  --port <n>               Port to listen on (default: 7744)
  --db <path>              Path to SQLite database file
`;

async function main() {
  if (values.help && !cmd) {
    console.log(HELP);
    return;
  }

  switch (cmd) {
    case "init": {
      const { init } = await import("./commands/init.js");
      await init(repoRoot);
      break;
    }
    case "add": {
      const { add } = await import("./commands/add.js");
      await add(repoRoot);
      break;
    }
    case "status": {
      const { status } = await import("./commands/status.js");
      await status(repoRoot);
      break;
    }
    case "index": {
      const { reindexRepo } = await import("./tools/reindex.js");
      const { spinner } = await import("@clack/prompts");
      const s = spinner();
      s.start("Indexing repo...");
      const summary = await reindexRepo(repoRoot, { force: values.force });
      const withSymbols = summary.added + summary.updated;
      if (summary.forced) {
        s.stop(
          `Full scan: ${summary.total} files scanned, ${withSymbols} with symbols` +
          (summary.skipped ? `, ${summary.skipped} skipped (no symbols)` : "") +
          ` (${summary.added} new, ${summary.updated} updated)`
        );
      } else {
        s.stop(
          `${summary.updated} updated, ${summary.added} added, ${summary.removed} removed` +
          `, ${summary.unchanged} unchanged` +
          (summary.skipped ? `, ${summary.skipped} skipped` : "")
        );
      }
      break;
    }
    case "drift": {
      const { checkDrift } = await import("./tools/drift.js");
      const failOnDrift = values["fail-on-drift"];
      const result = await checkDrift(repoRoot);
      console.log(result.summary);
      if (failOnDrift && result.drifted.length > 0) process.exit(1);
      break;
    }
    case "doctor":
    case "reformat": {
      const { doctor } = await import("./commands/doctor.js");
      const target = rest[0] ?? ".";
      await doctor(target, repoRoot, {
        dryRun: values["dry-run"],
        template: values.template,
      });
      break;
    }
    case "migrate": {
      const { migrate } = await import("./commands/migrate.js");
      await migrate(repoRoot);
      break;
    }
    case "serve": {
      const { serve } = await import("./commands/serve.js");
      await serve(repoRoot, {
        port: values.port ? parseInt(values.port, 10) : undefined,
        db: values.db,
      });
      break;
    }
    case "benchmark": {
      const subCmd = rest[0];
      if (!subCmd || values.help) {
        console.log(HELP);
        break;
      }
      if (subCmd === "doctor") {
        const { benchmarkDoctor } = await import("./commands/benchmark-doctor.js");
        // Support named params (--source / --dest) or legacy positional args
        const source = values.source ?? rest[1];
        const dest = values.dest ?? rest[2];
        if (!source || !dest) {
          console.error("Usage: sensei benchmark doctor --source <input-dir> --dest <output-dir> [--template <path>] [--examples <dir>] [--sample N] [--verbose]");
          process.exit(1);
        }
        await benchmarkDoctor(source, dest, repoRoot, {
          template: values.template,
          examples: values.examples,
          sample: values.sample ? parseInt(values.sample, 10) : undefined,
          verbose: values.verbose,
        });
      } else if (subCmd === "inspect") {
        const { benchmarkInspect } = await import("./commands/benchmark-inspect.js");
        const runBranch = rest[1];
        if (!runBranch) {
          console.error("Usage: sensei benchmark inspect <run>-<letter>");
          process.exit(1);
        }
        await benchmarkInspect(runBranch, repoRoot);
      } else if (subCmd === "promote") {
        const { benchmarkPromote } = await import("./commands/benchmark-promote.js");
        const runName = rest[1];
        if (!runName) {
          console.error("Usage: sensei benchmark promote <run-name>");
          process.exit(1);
        }
        await benchmarkPromote(runName, repoRoot);
      } else {
        console.error(`Unknown benchmark subcommand: ${subCmd}\n`);
        console.log(HELP);
        process.exit(1);
      }
      break;
    }
    default:
      if (cmd) console.error(`Unknown command: ${cmd}\n`);
      console.log(HELP);
  }
}

main().catch(err => {
  console.error(err.message);
  process.exit(1);
});
