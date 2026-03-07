#!/usr/bin/env node
import { parseArgs } from "node:util";

const { positionals, values } = parseArgs({
  args: process.argv.slice(2),
  allowPositionals: true,
  options: {
    force: { type: "boolean", default: false },
    "fail-on-drift": { type: "boolean", default: false },
    "dry-run": { type: "boolean", default: false },
    template: { type: "string" },
    examples: { type: "string" },
    sample: { type: "string" },   // number parsed manually
    port: { type: "string" },
    db: { type: "string" },
  },
});

const [cmd, ...rest] = positionals;

async function main() {
  switch (cmd) {
    case "init": {
      const { init } = await import("./commands/init.js");
      await init(process.cwd());
      break;
    }
    case "add": {
      const { add } = await import("./commands/add.js");
      await add(process.cwd());
      break;
    }
    case "status": {
      const { status } = await import("./commands/status.js");
      await status(process.cwd());
      break;
    }
    case "index": {
      const { reindexRepo } = await import("./tools/reindex.js");
      const { spinner } = await import("@clack/prompts");
      const s = spinner();
      s.start("Indexing repo...");
      const summary = await reindexRepo(process.cwd(), { force: values.force });
      if (summary.forced) {
        s.stop(`Full scan: ${summary.added} files indexed`);
      } else {
        s.stop(`${summary.updated} updated, ${summary.added} added, ${summary.removed} removed, ${summary.unchanged} unchanged`);
      }
      break;
    }
    case "drift": {
      const { checkDrift } = await import("./tools/drift.js");
      const failOnDrift = values["fail-on-drift"];
      const result = await checkDrift(process.cwd());
      console.log(result.summary);
      if (failOnDrift && result.drifted.length > 0) process.exit(1);
      break;
    }
    case "doctor":
    case "reformat": {
      const { doctor } = await import("./commands/doctor.js");
      const target = rest[0] ?? ".";
      await doctor(target, process.cwd(), {
        dryRun: values["dry-run"],
        template: values.template,
      });
      break;
    }
    case "migrate": {
      const { migrate } = await import("./commands/migrate.js");
      await migrate(process.cwd());
      break;
    }
    case "serve": {
      const { serve } = await import("./commands/serve.js");
      await serve(process.cwd(), {
        port: values.port ? parseInt(values.port, 10) : undefined,
        db: values.db,
      });
      break;
    }
    case "benchmark": {
      const subCmd = rest[0];
      if (subCmd === "doctor") {
        const { benchmarkDoctor } = await import("./commands/benchmark-doctor.js");
        const inputDir = rest[1];
        const outputName = rest[2];
        if (!inputDir || !outputName) {
          console.error("Usage: sensei benchmark doctor <input-dir> <output-name>");
          process.exit(1);
        }
        await benchmarkDoctor(inputDir, outputName, process.cwd(), {
          template: values.template,
          examples: values.examples,
          sample: values.sample ? parseInt(values.sample, 10) : undefined,
        });
      } else if (subCmd === "promote") {
        const { benchmarkPromote } = await import("./commands/benchmark-promote.js");
        const resultsDir = rest[1];
        if (!resultsDir) {
          console.error("Usage: sensei benchmark promote <results-dir>");
          process.exit(1);
        }
        await benchmarkPromote(resultsDir, process.cwd());
      } else {
        console.error(`Unknown benchmark subcommand: ${subCmd ?? "(none)"}`);
        process.exit(1);
      }
      break;
    }
    default:
      console.log(`sensei — AI skills toolchain

Commands:
  sensei init                   Set up a new repo (index + profiles + hook)
  sensei add                    Add sensei to an existing repo (non-destructive)
  sensei status                 Show index age, drift status, active profiles
  sensei index [--force]        Re-index the current repo
  sensei drift [--fail-on-drift] Check for doc drift
  sensei doctor <path> [--dry-run] [--template <path>]
                                Reformat docs to match canonical templates
  sensei migrate                Migrate agents/ folder to .index/checkpoints/
  sensei benchmark doctor <input-dir> <output-name> [--template <path>] [--examples <dir>] [--sample N] [--out <dir>]
                                Run 3-strategy doc conversion benchmark
  sensei benchmark promote <results-dir>
                                Review benchmark scores, capture preference, submit telemetry
  sensei serve [--port 7744] [--db <path>]
                                Start local telemetry report receiver
`);
  }
}

main().catch(err => {
  console.error(err.message);
  process.exit(1);
});
