#!/usr/bin/env bash
# Coverage for senseid's PURE (DB-free) modules.
#
# senseid is excluded from the main `cargo llvm-cov` job because its full suite
# needs a live `sensei_test` Postgres. But a large share of the crate is pure
# logic with NO DB dependency — language parsers, config adapters, assistant
# transcript parsing, ranking/classifiers/plan-graph/etc. Those modules' tests
# run WITHOUT a database, so we can measure them in CI and fold their coverage
# into the qlty aggregate.
#
# We select the pure modules' tests explicitly (so the run is green — no DB test
# is invoked), emit an lcov, then keep ONLY the pure modules' source files in the
# report. Adding a NEW pure module? Add it to PURE_MODULES below. The list is an
# allowlist: anything not listed is simply not measured here (never a DB file
# uploaded at 0%). If a name here ever collides with a DB test and the run hangs,
# the CI timeout catches it — a visible failure, never a silent miswrite.
#
# Usage: scripts/senseid-pure-coverage.sh <output-lcov-path>
set -euo pipefail

OUT="${1:-senseid-pure.lcov}"
RAW="$(mktemp)"

# Pure, DB-free senseid modules (verified: their tests run with no Postgres).
# Directory modules and single-file modules are treated the same for selection;
# the SF allowlist below distinguishes `mod/` from `mod.rs`.
PURE_MODULES=(
  adapters languages assistants installer libraries config
  agent_spawn classifiers git_identity ir maturity memory_slot model_insight
  model_provision observatory_home pattern_effectiveness plan_graph planner
  playbook project_overview ranking resolution review run_limits secret_scan
  stance verdicts
)

# cargo test filters: `<module>::` selects that root module's tests only.
FILTERS=()
for m in "${PURE_MODULES[@]}"; do FILTERS+=("${m}::"); done

echo "senseid-pure-coverage: running DB-free module tests (${#PURE_MODULES[@]} modules)…"
cargo llvm-cov -p senseid --no-fail-fast --lcov --output-path "$RAW" -- "${FILTERS[@]}"

# Keep only records whose SF is one of the pure modules (dir `src/<m>/` or file
# `src/<m>.rs`). Everything else — DB-touching code that wasn't exercised — is
# dropped so it can't drag the number down at 0%.
python3 - "$RAW" "$OUT" "${PURE_MODULES[@]}" <<'PY'
import re, sys
raw, out = sys.argv[1], sys.argv[2]
mods = sys.argv[3:]
alt = '|'.join(map(re.escape, mods))
keep_re = re.compile(rf'/senseid/src/(?:{alt})(?:/|\.rs$)')
rec, keep, kept = [], False, 0
with open(raw) as f, open(out, 'w') as w:
    for line in f:
        if line.startswith('SF:'):
            keep = bool(keep_re.search('/' + line[3:].strip()))
            rec = [line] if keep else []
        elif keep:
            rec.append(line)
            if line.startswith('end_of_record'):
                w.writelines(rec); kept += 1; keep = False; rec = []
print(f"senseid-pure-coverage: kept {kept} pure source files -> {out}", file=sys.stderr)
PY

rm -f "$RAW"
