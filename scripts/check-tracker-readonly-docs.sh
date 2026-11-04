#!/usr/bin/env bash
# The tracker read-only docs check (spec/daemon/tracker-metrics-writeback.md, gate 26).
#
# Trackers stopped being read-only on 2026-10-26: the local daemon writes usage metrics
# back to tickets assigned to the token owner. A document that still says trackers are
# never written contradicts the code, so every such statement must be narrowed, and a
# narrowed one names the spec.
#
#   check-tracker-readonly-docs.sh [path…]   check these files/dirs (default: the W-DOCS set)
#   check-tracker-readonly-docs.sh --test    self-check, so the check itself is verified
#
# Unit: each Markdown table row (a line starting with `|`) on its own, otherwise each
# blank-line-delimited paragraph, so one reference elsewhere in a table can't excuse a
# stale row. A unit is reported (file:line: text) when, lowercased, it says read-only,
# AND it is about trackers, AND it doesn't contain `tracker-metrics-writeback`. Only
# Markdown and `.feature` files are read; directories are walked for them.
set -uo pipefail

DEFAULT_SET=(
  docs/prd/modules/04-team
  docs/decisions.md
  docs/roadmap/09-dojo-expansion.md
  docs/design/integrations-and-gating-requirements.md
  docs/spec/dojo/daemon-sync.md
  docs/spec/daemon/tracker-jira-linkage.md
  docs/spec/screen/workspace-integrations.md
  docs/spec/screen/workspace-tickets.md
  docs/backlog.md
)

READONLY='read-only|read only|never writ|writes? back|no write path|nothing is changed|never changes anything|never written'
CONTEXT='tracker|jira|adapter|azure board|github issues|export|sync never|sync is|sync only'
REFERENCE='tracker-metrics-writeback'

# The files to read: each argument, or the Markdown and .feature files under it.
files_of() {
  local p
  for p in "$@"; do
    if [ -d "$p" ]; then
      find "$p" -type f \( -name '*.md' -o -name '*.feature' \) | sort
    elif [ -f "$p" ]; then
      case "$p" in *.md|*.feature) printf '%s\n' "$p" ;; esac
    else
      printf 'check-tracker-readonly-docs: no such file or directory: %s\n' "$p" >&2
      return 2
    fi
  done
}

# Every stale unit in one file, as file:line: text (the unit's first line).
check_file() {
  awk -v file="$1" -v ro="$READONLY" -v ctx="$CONTEXT" -v ref="$REFERENCE" '
    function flush() {
      if (unit != "") {
        u = tolower(unit)
        if (u ~ ro && u ~ ctx && index(u, ref) == 0) {
          printf "%s:%d: %s\n", file, start, substr(first, 1, 160)
        }
      }
      unit = ""
    }
    /^[[:space:]]*\|/ { flush(); unit = $0; start = NR; first = $0; flush(); next }
    /^[[:space:]]*$/  { flush(); next }
    {
      if (unit == "") { start = NR; first = $0 }
      unit = unit " " $0
    }
    END { flush() }
  ' "$1"
}

run_check() {
  local list f hits=0 out
  list=$(files_of "$@") || return 2
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    out=$(check_file "$f")
    if [ -n "$out" ]; then printf '%s\n' "$out"; hits=1; fi
  done <<< "$list"
  return $hits
}

self_test() {
  local dir fail=0 out
  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' RETURN
  cat > "$dir/table.md" <<'EOF'
| Surface | Rule |
|---|---|
| sync | Writes metrics back (spec/daemon/tracker-metrics-writeback.md). |
| adapters | Tracker adapters are strictly read-only. |
EOF
  cat > "$dir/members.md" <<'EOF'
Billing: read-only members are free.

A viewer has read-only access to the workspace.
EOF
  cat > "$dir/para.md" <<'EOF'
The Jira adapter
never writes to the tracker.

The Jira adapter writes back usage metrics only
(spec/daemon/tracker-metrics-writeback.md).
EOF
  cat > "$dir/scenario.feature" <<'EOF'
  Scenario: no export
    Then there is no export action and no write path
EOF
  printf 'Trackers are read-only.\n' > "$dir/ignored.txt"

  out=$(run_check "$dir")
  expect() { # <want: 1 = reported, 0 = not> <pattern> <label>
    if printf '%s\n' "$out" | grep -qF -- "$2"; then got=1; else got=0; fi
    if [ "$got" != "$1" ]; then printf 'FAIL: %s\n' "$3" >&2; fail=1; fi
  }
  expect 1 "table.md:4:" "a stale table row is reported though another row has the reference"
  expect 0 "table.md:3:" "a row with the reference is not reported"
  expect 0 "members.md" "read-only with no tracker context is not reported"
  expect 1 "para.md:1:" "a stale paragraph is reported at its first line"
  expect 0 "para.md:4:" "a paragraph with the reference is not reported"
  expect 1 "scenario.feature:1:" ".feature files are read"
  expect 0 "ignored.txt" "other files are not read"
  if [ "$(printf '%s\n' "$out" | wc -l)" != 3 ]; then
    printf 'FAIL: one line per stale unit (got: %s)\n' "$out" >&2; fail=1
  fi
  run_check "$dir/members.md" >/dev/null || { printf 'FAIL: a clean file exits 0\n' >&2; fail=1; }
  if run_check "$dir/table.md" >/dev/null; then printf 'FAIL: a hit exits non-zero\n' >&2; fail=1; fi
  run_check "$dir/missing.md" 2>/dev/null; [ $? = 2 ] || { printf 'FAIL: a missing path exits 2\n' >&2; fail=1; }
  [ $fail = 0 ] && printf 'check-tracker-readonly-docs: self-test passed\n'
  return $fail
}

if [ "${1:-}" = "--test" ]; then self_test; exit $?; fi
if [ $# -eq 0 ]; then
  cd "$(dirname "$0")/.." || exit 2
  set -- "${DEFAULT_SET[@]}"
fi
run_check "$@"
