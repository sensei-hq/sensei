#!/usr/bin/env bash
#
# Block transcript, log and personal data from entering the repository.
#
# This exists because it already happened. A contributor's Windows username and
# their employer's project path were lifted verbatim from a shared transcript
# into a test fixture, committed, and pushed to a public repo — along with two
# personal mailboxes, one belonging to a third party. Rewriting that out took a
# history rewrite and a force-push across three repositories.
#
# The fixtures needed the SHAPE, not the content. That distinction is what this
# script enforces.
#
# Scans STAGED content only, so it costs nothing on an unrelated commit. Fails
# closed: a hit blocks the commit and names the file and line, because a guard
# that warns and continues is a guard nobody reads.
#
# Usage:
#   check-no-leaks.sh          scan the staged changes (pre-commit)
#   check-no-leaks.sh --test   self-check, so the guard itself is verified
set -uo pipefail

RED=$'\033[0;31m'; YEL=$'\033[0;33m'; NC=$'\033[0m'

# Placeholder user names that are FINE in a path — the point is to allow
# synthetic fixtures while catching real home directories.
SAFE_USERS='user|users|dev|dev\.user|jane|john|alice|bob|acme|test|tester|example|runner|linuxbrew|home|root|USERNAME|<[a-z-]+>|\$\{?[A-Za-z_]+\}?'

# Domains that may legitimately appear. Everything else is treated as a real
# mailbox until someone adds it here deliberately.
SAFE_MAIL='sensei-hq\.com|example\.com|example-corp\.com|example\.org|acme[a-z-]*\.(com|co)|sensei\.test|users\.noreply\.github\.com|github\.com|anthropic\.com|jerrythomas\.name|[a-z]\.(co|dev)$'

fail=0
note() { printf '%s%s%s\n' "$RED" "$1" "$NC" >&2; fail=1; }

scan_paths() {
  # 1. Whole files that are transcripts or logs. `database/import/` is the
  #    project's own seed data and is the one legitimate home for .jsonl.
  local f
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in
      database/import/*) continue ;;
    esac
    case "$f" in
      *.jsonl|*.log|*events.jsonl|*chatSessions/*|*workspaceStorage/*|*/transcripts/*|*/facets/*)
        note "  $f — looks like a transcript or log; those belong outside the repo" ;;
    esac
  done <<< "$1"
}

scan_content() {
  local diff="$1" line file lineno text
  # Only added lines, with the file they landed in.
  file=""
  while IFS= read -r line; do
    case "$line" in
      '+++ b/'*) file="${line#+++ b/}"; lineno=0; continue ;;
      '@@'*) lineno=$(printf '%s' "$line" | sed -n 's/^@@ -[0-9,]* +\([0-9]*\).*/\1/p'); continue ;;
      '+'*) text="${line#+}"; lineno=$((lineno + 1)) ;;
      *) continue ;;
    esac
    [ -z "$file" ] && continue
    # This guard's own file is exempt: it necessarily contains strings shaped
    # like the things it detects. That exemption is a standing obligation —
    # every example below MUST be synthetic. The first version of this script
    # used a real contributor's username in its own self-test and the exemption
    # waved it straight through.
    case "$file" in .githooks/check-no-leaks.sh) continue ;; esac

    # 2. A real home directory: /Users/<name>, C:\Users\<name>, /home/<name>.
    if printf '%s' "$text" | grep -qiE "(/Users/|/home/|[Cc]:\\\\+Users\\\\+)" &&
       ! printf '%s' "$text" | grep -qiE "(/Users/|/home/|[Cc]:\\\\+Users\\\\+)($SAFE_USERS)([/\\\\\"' ]|$)"; then
      note "  $file:$lineno — real home directory in a path; use a placeholder"
      printf '    %s\n' "$(printf '%s' "$text" | cut -c1-100)" >&2
    fi

    # 3. A mailbox outside the allow-list.
    local mail
    mail=$(printf '%s' "$text" | grep -oiE '[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}' | head -1)
    if [ -n "$mail" ] && ! printf '%s' "$mail" | grep -qiE "@($SAFE_MAIL)"; then
      note "  $file:$lineno — email address '$mail' is not an allow-listed domain"
    fi

    # 4. Markers that only appear in a real assistant transcript.
    if printf '%s' "$text" | grep -qE '"(toolInvocationSerialized|tool\.execution_start|assistant\.turn_start)"' &&
       ! printf '%s' "$file" | grep -qE '\.(rs|ts|js|py|md)$'; then
      note "  $file:$lineno — raw transcript event content"
    fi
  done <<< "$diff"
}

run_scan() {
  local files diff
  files=$(git diff --cached --name-only --diff-filter=ACM)
  [ -z "$files" ] && return 0
  diff=$(git diff --cached --unified=0 -- $(printf '%s\n' "$files" | tr '\n' ' ') 2>/dev/null)
  scan_paths "$files"
  scan_content "$diff"
  return $fail
}

# ── Self-check ───────────────────────────────────────────────────────────────
# A guard that cannot fail is not a guard. This proves each rule catches its
# case AND lets the legitimate equivalent through.
self_test() {
  local pass=0 t=0
  check() { # name, text, expect(hit|clean)
    t=$((t + 1)); fail=0
    scan_content "$(printf '+++ b/src/x.rs\n@@ -0,0 +1 @@\n+%s' "$2")" 2>/dev/null
    if { [ "$3" = hit ] && [ $fail -eq 1 ]; } || { [ "$3" = clean ] && [ $fail -eq 0 ]; }; then
      pass=$((pass + 1)); printf '  ok    %s\n' "$1"
    else printf '  FAIL  %s (expected %s)\n' "$1" "$3"; fi
  }
  echo "check-no-leaks self-test"
  check "real mac home"      '"/Users/k.nakamura/Documents/app"'      hit
  check "real linux home"    '"/home/mjackson/src/app"'              hit
  check "windows home"       '"C:\\Users\\rkale\\work"'              hit
  check "placeholder home"   '"/Users/dev.user/Documents/app"'       clean
  check "generic home"       '"/home/user/project"'                  clean
  check "personal mailbox"   'contact: someone@icloud.com'           hit
  check "corporate mailbox"  'jane.doe@bigcorp.example.net'          hit
  check "allow-listed"       'hi@sensei-hq.com'                      clean
  check "example domain"     'dev@example-corp.com'                  clean
  fail=0
  scan_paths 'reports/facets/manoj/x.json'  2>/dev/null; [ $fail -eq 1 ] && { pass=$((pass+1)); echo "  ok    facets path"; } || echo "  FAIL  facets path"; t=$((t+1))
  fail=0
  scan_paths 'database/import/staging/models.jsonl' 2>/dev/null; [ $fail -eq 0 ] && { pass=$((pass+1)); echo "  ok    seed jsonl allowed"; } || echo "  FAIL  seed jsonl allowed"; t=$((t+1))
  echo "  $pass/$t passed"
  [ "$pass" -eq "$t" ]
}

if [ "${1:-}" = "--test" ]; then self_test; exit $?; fi

if ! run_scan; then
  printf '\n%sCommit blocked — the above looks like transcript, log or personal data.%s\n' "$YEL" "$NC" >&2
  printf 'Fixtures should copy the SHAPE, not the content. If a hit is wrong, add the\n' >&2
  printf 'case to SAFE_USERS / SAFE_MAIL in .githooks/check-no-leaks.sh with a reason.\n' >&2
  exit 1
fi
