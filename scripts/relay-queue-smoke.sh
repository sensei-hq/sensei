#!/usr/bin/env bash
# Relay autonomous-queue smoke (plan task 2.10) — proves the shipped daemon end
# to end against the LIVE local daemon and its `sensei` database: a coordinator
# mints a crew, two lanes join with no role and no crew_id, the person creates
# a sprint run over one backlog item, the lanes claim Task 0 / review it /
# unpack the plan, claim the two tasks (the second early-claim gated), report
# with branches on a throwaway git repository the run's config names, review
# each other's work, and the broker's merge gate merges both — the run
# completes on the sweeper's pass. Every step asserts the HTTP status and the
# row that should have moved.
#
# Leaves the daemon's rows behind (a real crew, closed at the end) and removes
# its temporary repository. Needs: the daemon on :7744, git, jq, curl.
#
#   scripts/relay-queue-smoke.sh            # ~3–4 min (two sweeper passes)
#   SENSEI_URL=http://127.0.0.1:7744 GATE_WAIT=150 scripts/relay-queue-smoke.sh
set -uo pipefail

URL=${SENSEI_URL:-http://127.0.0.1:7744}
GATE_WAIT=${GATE_WAIT:-150}
CT='content-type: application/json'
fail() { echo "FAIL: $*" >&2; cleanup; exit 1; }
cleanup() {
  if [ -n "${COORD_ID:-}" ] && [ -n "${CREW:-}" ]; then
    curl -s -o /dev/null -X POST "$URL/api/relay/crews/$CREW/close" \
      -H "Authorization: Bearer $COORD_TOKEN" -H "$CT" -d "{\"lane_id\":\"$COORD_ID\"}" || true
  fi
  [ -n "${REPO:-}" ] && rm -rf "$REPO"
}

# req METHOD PATH [TOKEN] [BODY] → prints "<status> <body>".
req() {
  local m=$1 p=$2 t=${3:-} b=${4:-} out code
  if [ -n "$b" ]; then
    out=$(curl -s -w '\n%{http_code}' -X "$m" "$URL$p" ${t:+-H "Authorization: Bearer $t"} -H "$CT" -d "$b")
  else
    out=$(curl -s -w '\n%{http_code}' -X "$m" "$URL$p" ${t:+-H "Authorization: Bearer $t"})
  fi
  code=${out##*$'\n'}; body=${out%$'\n'*}
  printf '%s %s' "$code" "$body"
}
status() { printf '%s' "$1" | cut -d' ' -f1; }
body() { printf '%s' "$1" | cut -d' ' -f2-; }
expect() { # RESPONSE STATUS WHAT
  [ "$(status "$1")" = "$2" ] || fail "$3: expected $2, got $(status "$1") $(body "$1")"
}
task_state() { body "$(req GET "/relay/tasks/$CREW/$1")" | jq -r '.task.state'; }
wait_state() { # TASK STATE SECONDS
  local i=0
  while [ $i -lt "$3" ]; do
    [ "$(task_state "$1")" = "$2" ] && return 0
    sleep 5; i=$((i + 5))
  done
  return 1
}

echo "== 0. daemon =="
H=$(req GET /health); expect "$H" 200 "health"
echo "   $(body "$H" | jq -c '{version, dbName}')"

echo "== 1. a throwaway repository the run's config names =="
REPO=$(mktemp -d "${TMPDIR:-/tmp}/sensei-queue-smoke.XXXXXX")
g() { git -C "$REPO" "$@" >/dev/null 2>&1 || fail "git $*"; }
g init -q -b main; g config user.email smoke@sensei.test; g config user.name smoke
echo base > "$REPO/README"; g add .; g commit -qm init
for t in t1 t2; do
  g checkout -qb "lane/$t" main; echo "$t" > "$REPO/$t.txt"; g add .; g commit -qm "$t"
done
g checkout -q main
echo "   $REPO on main, branches lane/t1 lane/t2"

echo "== 2. the coordinator mints the crew; two lanes join with no role and no crew_id =="
R=$(req POST /api/relay/register "" '{"role":"coordinator","family":"smoke","repo_key":"github.com/sensei-hq/smoke"}')
expect "$R" 200 "coordinator register"
COORD_ID=$(body "$R" | jq -r .lane_id); COORD_TOKEN=$(body "$R" | jq -r .lane_token); CREW=$(body "$R" | jq -r .crew_id)
[ "$(body "$R" | jq -r .guidance.version)" = "2" ] || fail "guidance v2"
for n in A B; do
  # No role, no crew_id: the lane joins the single active crew. On a machine
  # with other crews open the daemon refuses honestly (409
  # relay_crew_ambiguous) and the lane names the crew instead.
  R=$(req POST /api/relay/register "" "{\"family\":\"smoke\",\"suggested_name\":\"worker-$n\"}")
  if [ "$(status "$R")" = 409 ] && [ "$(body "$R" | jq -r .error)" = relay_crew_ambiguous ]; then
    echo "   (other crews are open on this machine: lane $n names crew_id)"
    R=$(req POST /api/relay/register "" "{\"family\":\"smoke\",\"suggested_name\":\"worker-$n\",\"crew_id\":\"$CREW\"}")
  fi
  expect "$R" 200 "lane $n register"
  [ "$(body "$R" | jq -r .crew_id)" = "$CREW" ] || fail "lane $n joined $(body "$R" | jq -r .crew_id), not $CREW"
  [ "$(body "$R" | jq -r .lane_title)" = "worker-$n" ] || fail "lane $n title"
  eval "LANE_$n=$(body "$R" | jq -r .lane_id); TOKEN_$n=$(body "$R" | jq -r .lane_token)"
done
echo "   crew=$CREW A=$LANE_A B=$LANE_B"

echo "== 3. the person drafts a sprint run over one backlog item, then starts it on the crew =="
R=$(req POST /relay/triage "" '{"title":"Queue smoke","kind":"gap","repo_key":"github.com/sensei-hq/smoke","tags":[]}')
expect "$R" 200 "triage"; ITEM=$(body "$R" | jq -r .item_id)
R=$(req POST /relay/runs "" "{\"title\":\"Smoke sprint\",\"goal\":\"Prove the queue\",\"item_ids\":[\"$ITEM\"],\"repo_key\":\"github.com/sensei-hq/smoke\",\"config\":{\"base_checkout\":\"$REPO\",\"base_branch\":\"main\",\"verify_cmd\":\"test -f README\",\"auto_merge\":true}}")
expect "$R" 201 "draft run"
RUN=$(body "$R" | jq -r .run_id)
[ "$(body "$R" | jq -r .status)" = draft ] || fail "run status: $(body "$R")"
[ "$(body "$R" | jq -r .planning_task_id)" = null ] || fail "a draft minted Task 0: $(body "$R")"
R=$(req POST "/relay/runs/$RUN/start" "" "{\"crew_id\":\"$CREW\"}")
expect "$R" 200 "start run"
TASK0=$(body "$R" | jq -r .planning_task_id)
[ "$(body "$R" | jq -r .status)" = planning ] || fail "started run status: $(body "$R")"
[ "$(body "$R" | jq -r .crew_id)" = "$CREW" ] || fail "started on $(body "$R" | jq -r .crew_id), not $CREW"
N=$(body "$(req GET /relay/now)")
echo "$N" | jq -e --arg r "$RUN" '.runs[] | select(.run_id == $r and .state == "planning" and .total_tasks == 1)' >/dev/null || fail "now.runs lacks the planning run: $(echo "$N" | jq -c .runs)"
echo "   run=$RUN task0=$TASK0 (now.runs shows it planning)"

echo "== 4. A claims Task 0 and reports a two-task plan; B claims the review and approves =="
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_A" "{\"lane_id\":\"$LANE_A\"}"); expect "$R" 200 "A claim_next"
[ "$(body "$R" | jq -r .claim_type)" = task ] && [ "$(body "$R" | jq -r .task.id)" = "$TASK0" ] || fail "A did not get Task 0: $(body "$R")"
PLAN='{"tasks":[{"key":"t1","title":"First file","phase":"impl","owned_paths":["t1.txt"],"acceptance":["t1.txt exists"],"deps":[]},{"key":"t2","title":"Second file","phase":"impl","owned_paths":["t2.txt"],"acceptance":["t2.txt exists"],"deps":["t1"]}]}'
REPORT_MD=$(printf 'Plan:\n\n```json\n%s\n```\n' "$PLAN" | jq -Rs .)
R=$(req POST "/api/relay/tasks/$TASK0/report" "$TOKEN_A" "{\"lane_id\":\"$LANE_A\",\"phase\":\"planning\",\"round\":1,\"summary\":{\"changed\":\"Planned two files.\"},\"report_md\":$REPORT_MD}")
expect "$R" 200 "Task 0 report"
N=$(body "$(req GET /relay/now)")
echo "$N" | jq -e --arg t "$TASK0" '.floating_reviews[] | select(.task_id == $t)' >/dev/null || fail "now.floating_reviews lacks Task 0"
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_B" "{\"lane_id\":\"$LANE_B\"}"); expect "$R" 200 "B claim_next"
[ "$(body "$R" | jq -r .claim_type)" = review ] && [ "$(body "$R" | jq -r .task.id)" = "$TASK0" ] || fail "B did not get the review: $(body "$R")"
N=$(body "$(req GET /relay/now)")
echo "$N" | jq -e --arg l "$LANE_B" --arg t "$TASK0" '.lanes[] | select(.id == $l and .reviewing.id == $t)' >/dev/null || fail "now.lanes lacks B reviewing Task 0"
R=$(req POST "/api/relay/tasks/$TASK0/review" "$TOKEN_B" "{\"lane_id\":\"$LANE_B\",\"verdict\":\"approved\",\"summary\":\"Sound plan.\"}")
expect "$R" 200 "Task 0 approval"; [ "$(body "$R" | jq -r .state)" = done ] || fail "Task 0 not done: $(body "$R")"
Q=$(body "$(req GET "/api/relay/queue?lane_id=$LANE_A" "$TOKEN_A")")
[ "$(echo "$Q" | jq '.queue | length')" = 2 ] || fail "queue should hold two ready tasks: $Q"
T1=$(echo "$Q" | jq -r '.queue[] | select(.title == "First file") | .task_id')
T2=$(echo "$Q" | jq -r '.queue[] | select(.title == "Second file") | .task_id')
echo "$Q" | jq -e --arg t "$T2" --arg d "$T1" '.queue[] | select(.task_id == $t and .claim_eligibility == "ready_claim_early" and .unmet_deps == [$d])' >/dev/null || fail "t2 should be claim-early on t1: $Q"
N=$(body "$(req GET /relay/now)")
echo "$N" | jq -e --arg r "$RUN" '.runs[] | select(.run_id == $r and .state == "active" and .ready_tasks == 2)' >/dev/null || fail "now.runs should show the run active with two ready tasks: $(echo "$N" | jq -c .runs)"
echo "   unpacked: t1=$T1 t2=$T2; run active"

echo "== 5. A and B claim one task each; B's early report is gated =="
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_A" "{\"lane_id\":\"$LANE_A\"}"); expect "$R" 200 "A claim_next"
[ "$(body "$R" | jq -r .task.id)" = "$T1" ] && [ "$(body "$R" | jq -r .task.deps_status)" = satisfied ] || fail "A should hold t1 satisfied: $(body "$R")"
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_B" "{\"lane_id\":\"$LANE_B\"}"); expect "$R" 200 "B claim_next"
[ "$(body "$R" | jq -r .task.id)" = "$T2" ] && [ "$(body "$R" | jq -r .task.deps_status)" = early_claim_gated ] || fail "B should hold t2 early: $(body "$R")"
V=$(body "$(req GET "/relay/tasks/$CREW/$T2")")
echo "$V" | jq -e --arg r "$RUN" --arg d "$T1" '.task | select(.run_id == $r and .deps_status == "early_claim_gated" and .unmet_deps == [$d])' >/dev/null || fail "task view should class t2 early: $(echo "$V" | jq -c .task)"
R=$(req POST "/api/relay/tasks/$T2/checkpoint" "$TOKEN_B" "{\"lane_id\":\"$LANE_B\",\"current_step\":\"scaffolding\",\"steps\":[{\"key\":\"scaffold\",\"title\":\"Scaffold t2\",\"state\":\"open\"}]}")
expect "$R" 200 "B checkpoint"
R=$(req POST "/api/relay/tasks/$T2/report" "$TOKEN_B" "{\"lane_id\":\"$LANE_B\",\"phase\":\"impl\",\"round\":1,\"summary\":{\"changed\":\"t2.txt\"},\"branch\":\"lane/t2\"}")
expect "$R" 409 "B early report"; [ "$(body "$R" | jq -r .error)" = relay_upstream_deps_unmet ] || fail "wrong 409: $(body "$R")"
echo "   t2's report gated on t1"

echo "== 6. A reports t1 on lane/t1; B claims the review (beside its own work) and approves; the gate merges =="
R=$(req POST "/api/relay/tasks/$T1/report" "$TOKEN_A" "{\"lane_id\":\"$LANE_A\",\"phase\":\"impl\",\"round\":1,\"summary\":{\"changed\":\"t1.txt\"},\"branch\":\"lane/t1\",\"files\":[{\"path\":\"t1.txt\",\"added\":1,\"removed\":0}]}")
expect "$R" 200 "A t1 report"; [ "$(body "$R" | jq -r .state)" = in_review ] || fail "t1 not in review"
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_B" "{\"lane_id\":\"$LANE_B\"}"); expect "$R" 200 "B claim_next (review)"
[ "$(body "$R" | jq -r .claim_type)" = review ] && [ "$(body "$R" | jq -r .task.id)" = "$T1" ] || fail "B should get t1's review: $(body "$R")"
R=$(req POST "/api/relay/tasks/$T1/review" "$TOKEN_B" "{\"lane_id\":\"$LANE_B\",\"verdict\":\"approved\",\"summary\":\"Good.\"}")
expect "$R" 200 "t1 approval"; [ "$(body "$R" | jq -r .state)" = approved ] || fail "t1 should hold approved for the gate: $(body "$R")"
echo "   waiting up to ${GATE_WAIT}s for the sweeper's gate pass…"
wait_state "$T1" done "$GATE_WAIT" || fail "t1 not merged by the gate within ${GATE_WAIT}s (state $(task_state "$T1"))"
V=$(body "$(req GET "/relay/tasks/$CREW/$T1")")
echo "$V" | jq -e '.history[] | select(.kind == "merged" and .lane_code == null)' >/dev/null || fail "t1 lacks the gate's merged row: $(echo "$V" | jq -c '[.history[].kind]')"
git -C "$REPO" merge-base --is-ancestor lane/t1 main || fail "lane/t1 not merged into main"
echo "   t1 merged by the gate ($(git -C "$REPO" log -1 --format=%h main))"

echo "== 7. B reports t2 now that t1 is done; A reviews; the gate merges; the run completes =="
R=$(req POST "/api/relay/tasks/$T2/report" "$TOKEN_B" "{\"lane_id\":\"$LANE_B\",\"phase\":\"impl\",\"round\":1,\"summary\":{\"changed\":\"t2.txt\"},\"branch\":\"lane/t2\",\"files\":[{\"path\":\"t2.txt\",\"added\":1,\"removed\":0}]}")
expect "$R" 200 "B t2 report"
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_A" "{\"lane_id\":\"$LANE_A\"}"); expect "$R" 200 "A claim_next (review)"
[ "$(body "$R" | jq -r .task.id)" = "$T2" ] || fail "A should get t2's review: $(body "$R")"
R=$(req POST "/api/relay/tasks/$T2/review" "$TOKEN_A" "{\"lane_id\":\"$LANE_A\",\"verdict\":\"approved\",\"summary\":\"Good.\"}")
expect "$R" 200 "t2 approval"
wait_state "$T2" done "$GATE_WAIT" || fail "t2 not merged by the gate within ${GATE_WAIT}s (state $(task_state "$T2"))"
git -C "$REPO" merge-base --is-ancestor lane/t2 main || fail "lane/t2 not merged into main"
i=0; until [ $i -ge "$GATE_WAIT" ]; do
  N=$(body "$(req GET /relay/now)")
  echo "$N" | jq -e --arg r "$RUN" '[.runs[] | select(.run_id == $r)] | length == 0' >/dev/null && break
  sleep 5; i=$((i + 5))
done
echo "$N" | jq -e --arg r "$RUN" '[.runs[] | select(.run_id == $r)] | length == 0' >/dev/null || fail "the run should have completed and left now.runs: $(echo "$N" | jq -c .runs)"
R=$(req POST /api/relay/tasks/claim_next "$TOKEN_A" "{\"lane_id\":\"$LANE_A\"}"); expect "$R" 200 "A claim_next (empty)"
[ "$(body "$R" | jq -r .claim_type)" = none ] || fail "nothing should be left to claim: $(body "$R")"
echo "   both tasks merged, run completed, queue empty"

echo "== 8. close the crew =="
cleanup; REPO=""; COORD_ID=""
echo "PASS: relay queue smoke — crew $CREW, run $RUN"
