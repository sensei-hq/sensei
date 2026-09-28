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
# Placeholder conventions this tree already uses, added when `--audit` flagged 8
# files that were never leaking: a SINGLE LETTER (`/Users/j`, `/Users/r`), an
# ellipsis (`/Users/...`), and the synthetic given names fixtures use. Kept
# separate from the list above so it is obvious these are shape-only stand-ins.
# A one-letter real username is not a thing; `k.nakamura` and `mjackson` stay
# OUT, because the self-test relies on them being caught.
SAFE_USERS="$SAFE_USERS|[a-z]|[a-z][a-z]|\.\.\.|keiko|name"
# NO USERNAME AT ALL is not a leaked username, and three shapes produce it:
#   /Users/.cursor/…      the user segment elided, leaving a dotfile
#   /home/.copilot/…      same
#   /Users/</code> …      prose or markup where the segment is a placeholder
# The last one is UI copy explaining that paths ARE redacted, so flagging it
# accused the redaction notice of being the leak.
SAFE_USERS="$SAFE_USERS|\.[a-z][a-z.]*|<[a-z/<>-]*|\{[a-z_]+\}|…"  # last is U+2026 …
#
# The DELIMITER after a safe user matters as much as the list. It was
# `[/\\"' ]`, which does not include a backtick — so `(`/Users/keiko` → …)`
# in prose read as a real home directory, because the placeholder was
# terminated by a character the rule had never heard of. Markdown is where
# most of these examples live, so backtick, paren, comma, semicolon and `>`
# are all ordinary terminators now.

# Domains that may legitimately appear. Everything else is treated as a real
# mailbox until someone adds it here deliberately.
SAFE_MAIL='sensei-hq\.com|example\.com|example-corp\.com|example\.org|acme[a-z-]*\.(com|co|dev)|sensei\.test|users\.noreply\.github\.com|github\.com|anthropic\.com|devuser\.name|[a-z]\.(co|dev)$'

# PRIVATE NAMES — a denylist that lives OUTSIDE the repository, because the list
# itself is the thing being protected. Committing "here are our clients" would
# leak exactly what it guards against.
#
# Default: $SENSEI_PRIVATE_NAMES, else ~/.sensei/private-names. One name per
# line, `#` comments and blanks ignored, matched case-insensitively as a
# substring. Absent file = this rule is skipped, silently — a developer who has
# not set one up must still be able to commit.
#
# WHY THIS EXISTS. An audit on 2026-09-27 found seven private names across 24
# tracked files of a PUBLIC repo — in production source comments and in test
# assertions — together with client codebase metrics ("8,647 files", "4,311
# nodes", "1,230 folders"). TWO OF THEM WERE COMMITTED AFTER THIS GUARD LANDED on
# 2026-08-26 — five and six days after. It did not catch them because it only ever
# checked the USERNAME segment of a path, and a client name used as a namespace or
# a package root (`<Client>.Policy.Services`, `com.<org>.<client>.service`) has no
# path in it at all.
PRIVATE_NAMES_FILE="${SENSEI_PRIVATE_NAMES:-$HOME/.sensei/private-names}"
private_re=''
if [ -r "$PRIVATE_NAMES_FILE" ]; then
  private_re=$(sed -e 's/#.*//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' \
                 "$PRIVATE_NAMES_FILE" | grep -v '^$' | paste -sd'|' -)
fi

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
      # The anonymised transcript corpus (Tier 2). These files ARE transcripts,
      # so the shape rule below would block every one of them — the carve-out is
      # deliberate and is exactly one directory deep.
      #
      # It is NOT a hole. Every CONTENT rule still runs over these files, and a
      # separate repo test requires each fixture to carry a `.meta.json` naming
      # the tool, its version and the anonymiser run that produced it. That is
      # the control: a fixture is machine-generated from an out-of-repo capture,
      # never hand-copied from a live session — which is the exact move that
      # caused the incident this whole script exists for.
      crates/senseid/tests/fixtures/transcripts/*) continue ;;
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
    # A DETECTOR NECESSARILY CONTAINS DETECTABLE STRINGS. This guard's own file
    # is exempt for that reason, and so is the anonymiser, whose self-test has to
    # prove it replaces a NON-placeholder username — `/Users/dev` would prove
    # nothing, since `dev` is on the safe list above.
    #
    # The exemption is a standing obligation: every example in both files MUST be
    # synthetic. The first version of this script used a real contributor's
    # username in its own self-test and the exemption waved it straight through.
    #
    # `audit_tree` grants NEITHER file an exemption, deliberately — that is what
    # caught the real client names this script's own comments once carried.
    case "$file" in
      .githooks/check-no-leaks.sh|scripts/anonymize-transcript.py) continue ;;
    esac

    # 2. A real home directory: /Users/<name>, C:\Users\<name>, /home/<name>.
      # The SAME two refinements `audit_tree` carries, and they must stay in step:
      # a home path never follows an alphanumeric (a route list such as
      # `logout/home/session` is not a home directory), and the delimiter after a
      # placeholder admits every ordinary terminator — markdown backticks above all,
      # since that is where these examples live.
    if printf '%s' "$text" | grep -qiE "(^|[^A-Za-z0-9])(/Users/|/home/|[Cc]:\\\\+Users\\\\+)" &&
       ! printf '%s' "$text" | grep -qiE "(^|[^A-Za-z0-9])(/Users/|/home/|[Cc]:\\\\+Users\\\\+)(($SAFE_USERS)([/\\\\\"'\` ),;>]|$)|[\"'\` ]|$)"; then
      note "  $file:$lineno — real home directory in a path; use a placeholder"
      printf '    %s\n' "$(printf '%s' "$text" | cut -c1-100)" >&2
    fi

    # 3. A mailbox outside the allow-list.
    local mail
    # 2b. A private name, from the out-of-repo denylist. Substring and
    #     case-insensitive on purpose: a client name in a namespace, the same
    #     name lowercased in a package path, and the same name again as a
    #     directory are ONE leak with three spellings. The name is NOT echoed —
    #     printing it would put the leak in CI logs and in every terminal
    #     scrollback that reads the failure.
    #     DIGESTS ARE NOT NAMES. A four-letter client name lands inside a
    #     checksum by chance — measured over this history, 130 times across
    #     Cargo.lock, bun.lock and a tsbuildinfo, every one inside an
    #     alphanumeric run of 32+ characters, while every genuine use sat in a
    #     run of four. So collapse long runs to a token BEFORE matching, rather
    #     than exempting those files by extension: an extension list is a
    #     second thing to keep in step with reality, and it would also blind
    #     the rule to a real name in a lockfile path.
    if [ -n "$private_re" ] &&
       printf '%s' "$text" | sed -E 's/[A-Za-z0-9]{32,}/<DIGEST>/g' | grep -qiE "($private_re)"; then
      note "  $file:$lineno — a name from your private-names denylist; use a placeholder"
    fi

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
  # A BARE PREFIX NAMES NOBODY. Source code that searches for the string
  # `"/Users/"` carries no username at all, which is the case the rule already
  # forgives for `/Users/.cursor/` and `/Users/</code>`. It blocked this repo's
  # own transcript-identity test, whose whole job is to find real home paths.
  check "bare prefix in code" 'for (at, _) in line.match_indices("/Users/") {'  clean
  check "bare prefix indexed" 'let rest = &line[at + "/Users/".len()..];'        clean
  check "bare prefix in prose" "it('leaves strings without /Users/ untouched')"  clean
  check "api route placeholder" 'via GET /users/{owner}), GitLab {group}'        clean
  check "still catches a user" 'let p = "/Users/rkale/notes";'                   hit
  # The windows form is the reason the no-username branch is NARROW. `\` is an
  # ordinary delimiter, so in `C:\\Users\\rkale` the SECOND backslash satisfies
  # it — a blanket-optional placeholder group excused a real username here.
  check "windows still caught"  '"C:\\Users\\mjackson\\src"'                     hit
  check "personal mailbox"   'contact: someone@icloud.com'           hit
  check "corporate mailbox"  'jane.doe@bigcorp.example.net'          hit
  check "allow-listed"       'hi@sensei-hq.com'                      clean
  check "example domain"     'dev@example-corp.com'                  clean
  # `acme` is the canonical placeholder organisation and was already allow-listed
  # on .com and .co. A fixture using acme.dev is the same synthetic name on a TLD
  # the list had simply never been extended to — it blocked a commit that only
  # renamed the local part of an address already in the tree.
  check "acme on .dev"       'email: dev.user@acme.dev'              clean
  check "real corp on .dev"  'email: someone@realcorp.dev'           hit

  # The denylist rule, with a denylist of its OWN so the test does not depend on
  # whoever is running it having one — and so it never reads the real list.
  local saved_re="$private_re"
  private_re='northgate|umbrella-corp'
  check "denylisted name"    'namespace Northgate.Policy.Services'   hit
  check "denylisted, cased"  '// over NORTHGATE 8,647 files'          hit
  check "denylisted in path" '"/Users/dev/Work/umbrella-corp/x"'      hit
  check "unlisted name"      'namespace Contoso.Policy.Services'      clean
  # A SHORT name collides inside a hash by pure chance: one four-letter client
  # name sat in a Cargo.lock checksum and another in a bun.lock sha512 — 130
  # such collisions across this history, every one inside an alphanumeric run
  # of 32+ characters, while every genuine use sat in a run of four. A digest
  # is not a leak, and a guard that says it is gets switched off.
  # The names below are SYNTHETIC, per the standing obligation above — `deca`
  # is spelled from hex digits so it can sit inside a real-shaped checksum.
  private_re='deca|zorp'
  check "name inside sha256" 'checksum = "c3aa5ce9deca49ac262752db7d1a4bda7b05b83eccd55559c7c20605447a6d9a"' clean
  check "name inside base64" '"integrity": "sha512-fT5mqqdzorpkEB2oFbTMDVdg1MGFxfQW"' clean
  check "same name as word"  'repos under Work/Deca and Developer/zorp' hit
  private_re="$saved_re"
  fail=0
  scan_paths 'reports/facets/dev/x.json'  2>/dev/null; [ $fail -eq 1 ] && { pass=$((pass+1)); echo "  ok    facets path"; } || echo "  FAIL  facets path"; t=$((t+1))
  fail=0
  scan_paths 'database/import/staging/models.jsonl' 2>/dev/null; [ $fail -eq 0 ] && { pass=$((pass+1)); echo "  ok    seed jsonl allowed"; } || echo "  FAIL  seed jsonl allowed"; t=$((t+1))
  # The anonymised transcript corpus. These files ARE transcripts — that is the
  # point of them — so the shape rule above would block every one. The carve-out
  # is exactly one directory, and it buys nothing on its own: the CONTENT rules
  # still run over these files, and `transcript_fixtures_carry_provenance` in
  # crates/senseid/tests/ requires each to have a `.meta.json` naming the tool,
  # its version and the anonymiser that produced it. A hand-copied session has
  # no such sibling, which is the check that matters here.
  fail=0
  scan_paths 'crates/senseid/tests/fixtures/transcripts/claude/tool-use.jsonl' 2>/dev/null
  [ $fail -eq 0 ] && { pass=$((pass+1)); echo "  ok    anonymised fixture allowed"; } || echo "  FAIL  anonymised fixture allowed"; t=$((t+1))
  fail=0
  scan_paths 'crates/senseid/src/transcripts/session.jsonl' 2>/dev/null
  [ $fail -eq 1 ] && { pass=$((pass+1)); echo "  ok    transcript elsewhere blocked"; } || echo "  FAIL  transcript elsewhere blocked"; t=$((t+1))
  echo "  $pass/$t passed"
  [ "$pass" -eq "$t" ]
}

# ── Whole-tree audit ─────────────────────────────────────────────────────────
# THE SECOND GAP the 2026-09-27 audit found: this guard has only ever seen
# STAGED diffs, so everything committed before it landed was never checked — and
# that is where most of what the audit found was sitting. `--audit` scans every
# tracked file instead, so the question "is the tree clean right now" has an
# answer that does not depend on what happens to be staged.
#
# Not run by the hook: it is O(tree) and a pre-commit hook must stay instant.
# Run it before a release, or after changing the denylist.
audit_tree() {
  local f n=0
  echo "check-no-leaks --audit: every tracked file"
  [ -z "$private_re" ] && printf '%s  no denylist at %s — name rules skipped%s\n' \
      "$YEL" "$PRIVATE_NAMES_FILE" "$NC"
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    # Same two rules as the staged path, applied to whole content. Binary files
    # are skipped by grep -I rather than filtered by extension: an extension
    # list is a second thing to keep in step with reality.
    # The digest collapse from `scan_content`, and it must stay in step with it.
    # `grep -qI .` keeps the binary skip: sed would happily read a binary file,
    # and piping it loses the -I that was doing that job.
    if [ -n "$private_re" ] && grep -qI . "$f" 2>/dev/null &&
       sed -E 's/[A-Za-z0-9]{32,}/<DIGEST>/g' "$f" 2>/dev/null | grep -qiE "($private_re)"; then
      note "  $f — contains a name from the private-names denylist"; n=$((n + 1))
    fi
    # PER LINE, not per file. This check used to ask "does the file contain a
    # home path AND contain no safe one anywhere" — two whole-file greps — so a
    # single `/Users/dev/…` excused every real home directory in the same file.
    # Proven: a file holding both was reported clean while a file holding only
    # the real one was flagged. The staged path has always been per line; this is
    # the drift those two comments keep warning about.
    #
    # The two detector files are exempt from THIS rule only, on the same standing
    # obligation as the staged path. The denylist rule above stays un-exempt for
    # every file — that is the rule that caught the real client names once sitting
    # in this script's own comments, and it must keep being able to.
    case "$f" in
      .githooks/check-no-leaks.sh|scripts/anonymize-transcript.py) ;;
      *)
        if grep -qI . "$f" 2>/dev/null; then
          while IFS= read -r hl; do
            [ -z "$hl" ] && continue
            note "  $f:${hl%%:*} — contains a real home directory"; n=$((n + 1))
          done < <(grep -niE "(^|[^A-Za-z0-9])(/Users/|/home/|[Cc]:\\+Users\\+)" "$f" 2>/dev/null |
                   grep -viE "(^|[^A-Za-z0-9])(/Users/|/home/|[Cc]:\\+Users\\+)(($SAFE_USERS)([/\\\"'\` ),;>]|$)|[\"'\` ]|$)")
        fi
        ;;
    esac
  done < <(git ls-files)
  if [ "$n" -eq 0 ]; then printf 'clean — no tracked file carries a private name or a real home path\n'; fi
  return $fail
}

if [ "${1:-}" = "--test" ]; then self_test; exit $?; fi
if [ "${1:-}" = "--audit" ]; then audit_tree; exit $?; fi

if ! run_scan; then
  printf '\n%sCommit blocked — the above looks like transcript, log or personal data.%s\n' "$YEL" "$NC" >&2
  printf 'Fixtures should copy the SHAPE, not the content. If a hit is wrong, add the\n' >&2
  printf 'case to SAFE_USERS / SAFE_MAIL in .githooks/check-no-leaks.sh with a reason.\n' >&2
  exit 1
fi
