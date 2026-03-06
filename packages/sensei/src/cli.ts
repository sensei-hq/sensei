#!/usr/bin/env node
import { parseArgs } from "node:util";

const { positionals } = parseArgs({
  args: process.argv.slice(2),
  allowPositionals: true,
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
      const s = (await import("@clack/prompts")).spinner();
      s.start("Indexing repo...");
      await reindexRepo(process.cwd());
      s.stop("Repo indexed.");
      break;
    }
    case "drift": {
      const { checkDrift } = await import("./tools/drift.js");
      const failOnDrift = rest.includes("--fail-on-drift");
      const result = await checkDrift(process.cwd());
      console.log(result.summary);
      if (failOnDrift && result.drifted.length > 0) process.exit(1);
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
  sensei init       Set up a new repo (index + profiles + hook)
  sensei add        Add sensei to an existing repo (non-destructive)
  sensei status     Show index age, drift status, active profiles
  sensei index      Re-index the current repo
  sensei drift      Check for doc drift
  sensei migrate    Migrate agents/ folder to .index/checkpoints/
`);
  }
}

main().catch(err => {
  console.error(err.message);
  process.exit(1);
});
