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
# uploaded at 0%).
#
# THE COLLISION THIS SCRIPT USED TO HAVE. A libtest filter is a SUBSTRING match
# on the whole test path, not a root-module match. So `libraries::` also selected
# `tasks::handlers::libraries::…` and `planner::` also selected
# `tasks::handlers::metrics::planner::…` — 33 tests that are not pure, 7 of which
# need Postgres and failed the job outright with "pool timed out". The comment
# here used to assert that `<module>::` "selects that root module's tests only".
# It does not, and that sentence was the bug.
#
# The fix is the SKIPS list below rather than a narrower filter, because the
# obvious narrowing does not work: skipping `::<m>::` (the module name appearing
# non-initially) looks like the right anchor and silently drops 30 genuinely
# pure tests, since `config` is BOTH a pure root module AND a real submodule of
# `adapters`. Skipping the colliding NON-PURE roots is exact — verified against
# the full 3,354-test listing: 0 pure tests lost, 0 collisions missed, 772 pure
# tests selected.
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

# Non-pure roots whose nested modules share a name with a pure module, so a
# substring filter reaches them. Skipped by root, which cannot touch a pure test
# (a pure test's path STARTS with a pure module, and neither of these is one).
# The post-run guard below is what catches a third one appearing.
SKIP_ROOTS=(api tasks)

# cargo test filters: `<module>::` is a SUBSTRING match, so it selects that root
# module's tests AND anything nested anywhere that happens to share the name.
# The SKIP_ROOTS above remove the latter.
FILTERS=()
for m in "${PURE_MODULES[@]}"; do FILTERS+=("${m}::"); done
for r in "${SKIP_ROOTS[@]}"; do FILTERS+=(--skip "${r}::"); done

echo "senseid-pure-coverage: running DB-free module tests (${#PURE_MODULES[@]} modules)…"
RUN_LOG="$(mktemp)"
cargo llvm-cov -p senseid --no-fail-fast --lcov --output-path "$RAW" \
  -- "${FILTERS[@]}" 2>&1 | tee "$RUN_LOG"

# GUARD: every test that actually ran must have a pure root module. Without this
# a new DB module nested under a new root re-introduces the collision silently —
# and it only announces itself if it happens to need Postgres. Assert the
# property instead of relying on the symptom.
python3 - "$RUN_LOG" "${PURE_MODULES[@]}" <<'PY'
import re, sys
log, mods = sys.argv[1], set(sys.argv[2:])
ran = re.findall(r'^test ([\w:]+) \.\.\.', open(log, errors='replace').read(), re.M)
impure = sorted({t for t in ran if t.split('::')[0] not in mods})
if impure:
    print(f"\nsenseid-pure-coverage: {len(impure)} NON-PURE test(s) ran — the "
          f"filters reached outside the allowlist:", file=sys.stderr)
    for t in impure[:20]:
        print(f"    {t}", file=sys.stderr)
    print("Add that test's ROOT module to SKIP_ROOTS in this script.", file=sys.stderr)
    sys.exit(1)
print(f"senseid-pure-coverage: {len(ran)} tests ran, all with a pure root module.",
      file=sys.stderr)
PY
rm -f "$RUN_LOG"

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
