#!/usr/bin/env bash
# Test sensei-mcp via JSON-RPC over stdio.
# Requires: senseid running on :7744, sensei-mcp binary built.
set -euo pipefail

PASS=0
FAIL=0
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MCP_BIN="${SENSEI_MCP_BIN:-$SCRIPT_DIR/../../../target/release/sensei-mcp}"

if [ ! -x "$MCP_BIN" ]; then
  MCP_BIN="$SCRIPT_DIR/../../../target/debug/sensei-mcp"
fi
if [ ! -x "$MCP_BIN" ]; then
  echo "SKIP: sensei-mcp binary not found. Build first: cargo build -p sensei-mcp"
  exit 0
fi

if ! curl -s --connect-timeout 2 http://127.0.0.1:7744/health >/dev/null 2>&1; then
  echo "SKIP: senseid daemon not running on :7744"
  exit 0
fi

# Send JSON-RPC messages, return Nth response line (1-indexed)
mcp_call() {
  local messages="$1"
  local line_num="${2:-1}"
  echo "$messages" | timeout 10 "$MCP_BIN" 2>/dev/null | sed -n "${line_num}p"
}

run_test() {
  local name="$1"
  local response="$2"
  local check="$3"

  if echo "$response" | python3 -c "$check" 2>/dev/null; then
    echo "  PASS $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL $name"
    echo "       Response: $(echo "$response" | head -c 200)"
    FAIL=$((FAIL + 1))
  fi
}

INIT_MSGS='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

echo "=== MCP Protocol Tests ==="

# ── Initialize + tools/list ──────────────────────────────────────────────────
INIT_RESP=$(mcp_call "$INIT_MSGS" 1)
TOOLS_RESP=$(mcp_call "$INIT_MSGS" 2)

run_test "initialize returns protocolVersion" "$INIT_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert d['result']['protocolVersion'] == '2024-11-05'"

run_test "initialize returns serverInfo" "$INIT_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert d['result']['serverInfo']['name'] == 'sensei'"

run_test "tools/list returns array" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert isinstance(d['result']['tools'], list)"

run_test "has search tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'search' in names"

run_test "has get_callers tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'get_callers' in names"

run_test "has get_callees tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'get_callees' in names"

run_test "has get_patterns tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'get_patterns' in names"

run_test "has get_lib_docs tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'get_lib_docs' in names"

run_test "has update_phase tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'update_phase' in names"

run_test "has get_workflow_state tool" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'get_workflow_state' in names"

run_test "at least 14 tools registered" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); n=len(d['result']['tools']); assert n >= 14, f'Only {n}'"

run_test "update_phase has phase as required param" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); t=[x for x in d['result']['tools'] if x['name']=='update_phase'][0]; assert 'phase' in t['inputSchema']['required']"

run_test "get_workflow_state has no required params" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); t=[x for x in d['result']['tools'] if x['name']=='get_workflow_state'][0]; assert t['inputSchema']['required'] == []"

# ── update_phase call ────────────────────────────────────────────────────────
echo ""
echo "=== tools/call: update_phase ==="

UPDATE_MSGS='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"update_phase","arguments":{"phase":"build","task":"MCP protocol test","issue":"999"}}}'

UPDATE_RESP=$(mcp_call "$UPDATE_MSGS" 2)

run_test "update_phase returns result with content" "$UPDATE_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert 'content' in d['result']"

run_test "update_phase content has ok:true" "$UPDATE_RESP" \
  "import sys,json; d=json.load(sys.stdin); text=d['result']['content'][0]['text']; data=json.loads(text); assert data.get('ok') == True"

# ── get_workflow_state call ──────────────────────────────────────────────────
echo ""
echo "=== tools/call: get_workflow_state ==="

STATE_MSGS='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_workflow_state","arguments":{}}}'

STATE_RESP=$(mcp_call "$STATE_MSGS" 2)

run_test "get_workflow_state returns content" "$STATE_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert 'content' in d['result']"

run_test "get_workflow_state has active_phase" "$STATE_RESP" \
  "import sys,json; d=json.load(sys.stdin); text=d['result']['content'][0]['text']; data=json.loads(text); assert 'active_phase' in data"

run_test "get_workflow_state has updated_at" "$STATE_RESP" \
  "import sys,json; d=json.load(sys.stdin); text=d['result']['content'][0]['text']; data=json.loads(text); assert 'updated_at' in data or data.get('active_phase') is None"

# ── log_event call ───────────────────────────────────────────────────────────
echo ""
echo "=== tools/call: log_event ==="

LOG_MSGS='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"log_event","arguments":{"type":"phase_transition","data":"{\"from\":\"ideate\",\"to\":\"analyze\"}"}}}'

LOG_RESP=$(mcp_call "$LOG_MSGS" 2)

run_test "log_event returns content" "$LOG_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert 'content' in d['result']"

run_test "log_event content has ok:true" "$LOG_RESP" \
  "import sys,json; d=json.load(sys.stdin); text=d['result']['content'][0]['text']; data=json.loads(text); assert data.get('ok') == True"

run_test "log_event returns id" "$LOG_RESP" \
  "import sys,json; d=json.load(sys.stdin); text=d['result']['content'][0]['text']; data=json.loads(text); assert 'id' in data"

run_test "has log_event tool in tools/list" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); names=[t['name'] for t in d['result']['tools']]; assert 'log_event' in names"

run_test "at least 15 tools registered" "$TOOLS_RESP" \
  "import sys,json; d=json.load(sys.stdin); n=len(d['result']['tools']); assert n >= 15, f'Only {n}'"

# ── error handling ───────────────────────────────────────────────────────────
echo ""
echo "=== Error handling ==="

BAD_MSGS='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}'

BAD_RESP=$(mcp_call "$BAD_MSGS" 2)

run_test "unknown tool returns content (not crash)" "$BAD_RESP" \
  "import sys,json; d=json.load(sys.stdin); assert 'result' in d"

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
