#!/usr/bin/env bash
#
# Sensei-plane target resolution for run.sh (L0 ruling 2026-11-05, after the
# stray `relay` schema in the live daemon DB): the sensei plane must never
# target the live daemon DB. It runs against a lane-owned test database —
# TEST_DATABASE_URL, an explicit non-live SENSEI_DATABASE_URL, or a
# sensei_test_<lane> derived from the branch and created from the DDL tree.
#
# Pure functions (unit-tested by self-test-apply.sh, no database needed):
#   sensei_is_live <url>      exit 0 iff the URL names the live daemon DB
#   sensei_lane_name [branch] print the sanitized lane name, or fail
# Effectful (integration-tested):
#   sensei_ensure_db <url>    create + deploy the lane DB when missing
#   sensei_resolve_url        print the sensei-plane URL, refusing live (exit 2)
#
# macOS ships bash 3.2: no associative arrays, no ${var,,}, no mapfile.
# This file is sourced, never executed.
#
# Boundary (documented, not silent): the live check covers literal loopback
# hosts (localhost, 127/8, [::1]), empty/unix-socket hosts incl. the %2F
# form, and ports 5432 (incl. default) + 7744. A DHCP hostname resolving to
# localhost, double-encoded hosts, or dbname defaulting from a user literally
# named `sensei` are out of scope: lanes are cooperating agents, and the
# refusal targets accidental routing, not deliberate evasion.

# Lowercase via tr (bash 3.2 has no ${var,,}).
_sensei_lower() {
    printf '%s' "$1" | tr 'A-Z' 'a-z'
}

# Split a postgres URL into host|port|dbname on stdout. Returns nonzero when
# the URL has no parseable authority/path (the caller lets psql report that).
# - scheme: anything before :// (case-insensitive, unvalidated otherwise)
# - userinfo: stripped on the LAST @ (a password may hold one)
# - host: [bracketed] or up to the last : (numeric tail = port); %2F-decoded
# - port: empty means the 5432 default
# - dbname: first path segment, trailing slashes stripped, empty means $USER
sensei_split_url() {
    local rest auth path hostport host tail port dbname
    rest="$1"
    # Strip the scheme.
    case "$rest" in
        *://*) rest="${rest#*://}" ;;
        *) return 1 ;;
    esac
    # Strip the query.
    rest="${rest%%\?*}"
    # Split authority from path on the first /.
    case "$rest" in
        */*) auth="${rest%%/*}"; path="${rest#*/}" ;;
        *) auth="$rest"; path="" ;;
    esac
    # Strip userinfo on the LAST @: user@host vs pass-word with @.
    case "$auth" in
        *@*) hostport="${auth##*@}" ;;
        *) hostport="$auth" ;;
    esac
    # Query-form host (postgresql:///db?host=... was stripped above with the
    # query — re-read it): only the socket case matters, and an empty host
    # already reads local, so nothing more to do.
    # Bracketed IPv6 vs host[:port] on the last colon.
    case "$hostport" in
        \[*\]*)
            host="${hostport%%\]*}"
            host="${host#\[}"
            tail="${hostport#*\]}"
            case "$tail" in
                :*) port="${tail#:}" ;;
                *) port="" ;;
            esac
            ;;
        *:*)
            tail="${hostport##*:}"
            case "$tail" in
                *[!0-9]*|"") host="$hostport"; port="" ;;
                *) host="${hostport%:*}"; port="$tail" ;;
            esac
            ;;
        *) host="$hostport"; port="" ;;
    esac
    # Decode an encoded socket host (%2F, either case).
    host="$(printf '%s' "$host" | sed -e 's/%2F/\//g' -e 's/%2f/\//g')"
    host="$(_sensei_lower "$host")"
    [ -z "$port" ] && port="5432"
    # First path segment, trailing slashes stripped.
    dbname="${path%%/*}"
    while [ "${dbname%/}" != "$dbname" ]; do dbname="${dbname%/}"; done
    [ -z "$dbname" ] && dbname="${USER:-}"
    printf '%s|%s|%s' "$host" "$port" "$dbname"
}

# Exit 0 iff the URL targets the live daemon DB: dbname `sensei` (any case —
# libpq matches it exactly, so a differently-cased name fails closed at
# connect time either way) on a local transport (loopback, empty/socket
# host) at port 5432 (incl. default) or the daemon's own 7744. Unparsable
# URLs are not live (psql reports them); remote hosts are the caller's
# deliberate target, not the local daemon DB.
sensei_is_live() {
    local parts host rest port dbname
    parts="$(sensei_split_url "$1")" || return 1
    host="${parts%%|*}"; rest="${parts#*|}"
    port="${rest%%|*}"; dbname="${rest#*|}"
    case "$(_sensei_lower "$dbname")" in
        sensei) : ;;
        *) return 1 ;;
    esac
    case "$host" in
        localhost|127.*|::1|""|/*) : ;;
        *) return 1 ;;
    esac
    case "$port" in
        5432|7744) return 0 ;;
        *) return 1 ;;
    esac
}

# Print the sanitized lane name for a branch (arg, else current git branch):
# strip a lane/ prefix, lowercase, anything-not-alnum becomes _, cap 51 chars
# (sensei_test_ + 51 fits Postgres' 63). Fails on empty/detached/no git.
sensei_lane_name() {
    local branch name
    if [ $# -gt 0 ]; then branch="$1"; else branch="$(git branch --show-current 2>/dev/null || true)"; fi
    [ -n "$branch" ] || return 1
    case "$branch" in
        lane/*) branch="${branch#lane/}" ;;
    esac
    name="$(printf '%s' "$branch" | tr 'A-Z' 'a-z' | tr -c 'a-z0-9' '_' | cut -c1-51)"
    [ -n "$name" ] || return 1
    printf '%s' "$name"
}

sensei_lane_db_url() {
    printf 'postgresql://localhost:5432/sensei_test_%s' "$(sensei_lane_name "$@")"
}

# Create the lane DB when missing: createdb + the DDL tree (dbd deploy,
# scope default, from database/) + a connectivity proof. No-op when it
# already answers. Exit 127 when a tool is missing, 69 when the target ends
# unusable. Never stamps (the Rust template flow owns stamping).
sensei_ensure_db() {
    local url tool dbname libdir
    url="$1"
    if psql "$url" -X -q -c 'select 1' >/dev/null 2>&1; then
        return 0
    fi
    for tool in createdb dbd; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "run.sh: cannot create lane database: $tool not found on PATH." >&2
            return 127
        fi
    done
    dbname="${url##*/}"; dbname="${dbname%%\?*}"
    echo "run.sh: creating lane database $dbname from the DDL tree…" >&2
    createdb -h localhost "$dbname" >/dev/null 2>&1 || {
        echo "run.sh: cannot create lane database $dbname (createdb failed)." >&2
        return 69
    }
    libdir="$(dirname "${BASH_SOURCE[0]:-$0}")"
    (cd "$libdir/../.." && DATABASE_URL="$url" dbd deploy --scope default >/dev/null 2>&1) || {
        echo "run.sh: cannot create lane database $dbname (dbd deploy failed)." >&2
        return 69
    }
    psql "$url" -X -q -c 'select 1' >/dev/null 2>&1 || {
        echo "run.sh: cannot create lane database $dbname (unreachable after deploy)." >&2
        return 69
    }
}

# Print the sensei-plane URL: TEST_DATABASE_URL, else an explicit
# SENSEI_DATABASE_URL, else the branch-derived lane DB (created). Refuses the
# live daemon DB (exit 2) whatever named it — even an explicit variable.
sensei_resolve_url() {
    local url lane
    if [ -n "${TEST_DATABASE_URL:-}" ]; then
        url="$TEST_DATABASE_URL"
    elif [ -n "${SENSEI_DATABASE_URL:-}" ]; then
        url="$SENSEI_DATABASE_URL"
    else
        lane="$(sensei_lane_name 2>/dev/null)" || {
            echo "run.sh: cannot derive a lane database (not on a branch — detached HEAD or no git)." >&2
            echo "  Set TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test_<lane> to a lane-owned test database." >&2
            return 2
        }
        url="postgresql://localhost:5432/sensei_test_$lane"
        sensei_ensure_db "$url" || return $?
    fi
    if sensei_is_live "$url"; then
        echo "run.sh: refusing the live daemon DB ($url)." >&2
        echo "  The sensei plane runs only against a lane-owned test database." >&2
        echo "  Set TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test_<lane> (createdb + dbd deploy), or run on a branch so run.sh derives one." >&2
        return 2
    fi
    printf '%s' "$url"
}
