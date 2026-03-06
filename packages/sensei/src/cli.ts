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
`);
  }
}

main().catch(err => {
  console.error(err.message);
  process.exit(1);
});
