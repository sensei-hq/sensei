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
    mcp: { type: "boolean", default: false },
    template: { type: "string" },
    examples: { type: "string" },
    sample: { type: "string" },   // number parsed manually
    port: { type: "string" },
    db: { type: "string" },
    // benchmark doctor named params
    source: { type: "string" },
    dest: { type: "string" },
    verbose: { type: "boolean", default: false },
    repo: { type: "string" },
    // stats command
    tool: { type: "string" },
    session: { type: "string" },
    since: { type: "string" },
    all: { type: "boolean", default: false },
    json: { type: "boolean", default: false },
    gaps: { type: "boolean", default: false },
    hooks: { type: "boolean", default: false },
    drift: { type: "boolean", default: false },
    agent: { type: "string" },
    lib: { type: "string" },
    global: { type: "boolean", default: false },
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

const HELP = `sensei — AI skills toolchain

Usage:
  sensei <command> [options]

Commands:
  init                     Set up a new repo (index + profiles + hook)
  add                      Add sensei to an existing repo (non-destructive)
  setup --mcp              Register sensei MCP server in ~/.claude/mcp.json
  setup --hooks            Install Claude hook scripts and register daemon autostart
  status                   Show index age, drift status, active profiles
  index                    Re-index the current repo
  drift                    Check for doc drift
  doctor <path>            Reformat docs to match canonical templates
  migrate                  Migrate agents/ folder to .sensei/checkpoints/
  benchmark doctor         Run 3-strategy doc conversion benchmark
  benchmark coverage       Score llmspec.yaml docs[].covers[] with Ollama
  benchmark populate       A/B benchmark: Claude without vs with populate-llmspec skill
  benchmark inspect        Switch to a benchmark branch for inspection
  benchmark promote        Merge chosen strategy branch, submit telemetry
  serve                    Start local telemetry report receiver
  server status            Check if server is running and show model setup status
  stats                    Show tool usage analytics (last 7 days)
  update-registry          Index custom_libs from .sensei/config.yaml into Supabase
  update-registry --lib <name>   Re-index a single named library
  update-registry --global --lib <name>   Promote lib to shared pool (all repos can link to it)
  install-skills           Install bundled sensei skills for Claude Code (prompts for repo/global)
  install-skills --global  Install all skills globally (~/.claude/skills/)

Options:
  -h, --help               Show this help message

setup:
  --mcp                    Register MCP server (writes ~/.claude/mcp.json)
  --agent <name>           Generate and install project-specific skills (supported: claude)
  --hooks                  Install Claude hook scripts and register daemon autostart

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

benchmark coverage:
  Uses local Ollama model to populate llmspec.yaml docs[].covers[]
  and score against .sensei/llmspec-expected.yaml gold standard

benchmark indexer:
  Compare cocoindex-code vs sensei's symbol indexer.
  Measures file coverage, query relevance, and prints a spot-check for manual review.
  Requires: cocoindex-code installed (pipx install cocoindex-code) and indexed.

benchmark populate:
  Compares Claude-without-skill vs Claude-with-populate-llmspec-skill.
  Scores each strategy with score-coverage.ts and reports tokens, time, score.
  Requires: .sensei/llmspec.yaml and .sensei/llmspec-expected.yaml

benchmark inspect:
  sensei benchmark inspect <run>-<a|b|c>

benchmark promote:
  sensei benchmark promote <run-name>

serve:
  --port <n>               Port to listen on (default: 7744)
  --db <path>              Path to SQLite database file

watch:
  --repo <path>            Repo to watch (default: auto-detected repo root)
  --drift                  Run drift check after each reindex (silent when clean)

stats:
  --all                    Show all-time data instead of last 7 days
  --tool <name>            Show stats for a specific tool
  --session <id>           Show all events for a session
  --since <YYYY-MM-DD>     Show events on or after this date
  --json                   Output as JSON
  --gaps                   Show missed-opportunity report (bash vs sensei tools)
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
    case "setup": {
      if (values.agent) {
        const { setupAgent } = await import("./commands/setup.js");
        await setupAgent(repoRoot, values.agent);
        break;
      }
      if (values.hooks) {
        const { setupHooks } = await import("./commands/setup.js");
        await setupHooks();
        break;
      }
      if (!values.mcp) {
        console.error("Usage: sensei setup --mcp");
        process.exit(1);
      }
      const { setupMcp } = await import("./commands/setup.js");
      const { createRequire } = await import("module");
      const { dirname: _dirname, join: _join } = await import("path");
      const _require = createRequire(import.meta.url);
      const mcpPkgPath = _require.resolve("@sensei/mcp/package.json");
      const mcpIndexJs = _join(_dirname(mcpPkgPath), "dist", "index.js");
      await setupMcp(repoRoot, mcpIndexJs);
      break;
    }
    case "status": {
      const { status } = await import("./commands/status.js");
      await status(repoRoot);
      break;
    }
    case "index": {
      const { reindexRepo } = await import("@sensei/tools");
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
      const { checkDrift } = await import("@sensei/tools");
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
      } else if (subCmd === "coverage") {
        const { benchmarkCoverage } = await import("./commands/benchmark-coverage.js");
        await benchmarkCoverage(repoRoot);
      } else if (subCmd === "populate") {
        const { benchmarkPopulate } = await import("./commands/benchmark-populate.js");
        await benchmarkPopulate(repoRoot);
      } else if (subCmd === "indexer") {
        const { benchmarkIndexer } = await import("./commands/benchmark-indexer.js");
        await benchmarkIndexer(repoRoot);
      } else {
        console.error(`Unknown benchmark subcommand: ${subCmd}\n`);
        console.log(HELP);
        process.exit(1);
      }
      break;
    }
    case "server": {
      const subCmd = rest[0];
      if (subCmd === "status" || !subCmd) {
        interface HealthResponse {
          backend?: string;
          ollamaRunning?: boolean;
        }
        interface SetupResponse {
          diskFreeGB?: number;
          ramAvailableGB?: number;
          ollamaModel?: boolean;
          ollamaModelName?: string;
        }
        const url = process.env.SENSEI_SERVER_URL ?? "http://localhost:7744";
        try {
          const res = await fetch(`${url}/health`, { signal: AbortSignal.timeout(2000) });
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          const data = await res.json() as HealthResponse;
          console.log(`sensei server: running at ${url}`);
          console.log(`  backend: ${data.backend ?? "none"}`);
          console.log(`  ollama:  ${data.ollamaRunning ? "running" : "not running"}`);
          const setupRes = await fetch(`${url}/setup/status`, { signal: AbortSignal.timeout(2000) });
          if (!setupRes.ok) throw new Error(`HTTP ${setupRes.status}`);
          const setup = await setupRes.json() as SetupResponse;
          console.log(`  disk:    ${setup.diskFreeGB} GB free`);
          console.log(`  ram:     ${setup.ramAvailableGB} GB available`);
          console.log(`  model:   ${setup.ollamaModel ? `✓ ${setup.ollamaModelName}` : `✗ ${setup.ollamaModelName} not pulled`}`);
        } catch {
          console.log(`sensei server: not running at ${url}`);
          console.log(`  Start with: sensei serve`);
        }
        break;
      }
      console.error(`Unknown server subcommand: ${subCmd}`);
      process.exit(1);
    }
    case "watch": {
      const { watch } = await import("./commands/watch.js");
      const repo = values.repo ?? repoRoot;
      await watch(repo, { drift: values.drift });
      break;
    }
    case "stats": {
      const { stats } = await import("./commands/stats.js");
      await stats({
        all: values.all,
        tool: values.tool,
        session: values.session,
        since: values.since,
        json: values.json,
        gaps: values.gaps,
      });
      break;
    }
    case "update-registry": {
      const { updateRegistry } = await import("./commands/update-registry.js");
      await updateRegistry(repoRoot, values.lib, { global: values.global });
      break;
    }
    case "install-skills": {
      const { installSkills, promptAndInstallSkills } = await import("./commands/install-skills.js");
      if (values.global) {
        await installSkills(repoRoot, "global");
      } else if (positionals.length > 1) {
        // sensei install-skills <repo-path>
        await installSkills(positionals[1], "repo");
      } else {
        await promptAndInstallSkills(repoRoot);
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
