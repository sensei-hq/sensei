// Usage at the source (#238 step 2). Claude Code's hook payloads carry no
// token or cost figures; the engine's own events do. This module forwards
// them to the daemon's /hook/event endpoint as two event kinds:
//
//   SessionMeasure — the context window's fill, the rate-limit windows and
//                    the session's cost as /cost totals it (list price).
//   TurnComplete   — one turn's four token counts and the model that answered.
//
// Delivery mirrors _lib.sh's send_telemetry: POST to the daemon, and on any
// failure dead-letter the line to ~/.sensei/events.jsonl, which the daemon's
// capture_drain imports. Each line carries its own `ts`, so a drained event
// keeps the time it happened.
import type { EngineInterface, Register } from 'claude-code'

// The POST's own bound, so a hung daemon cannot hold the hook to its budget.
const POST_TIMEOUT_MS = 2000

type Fields = Record<string, unknown>

const daemonUrl = async ($: EngineInterface) =>
  (await $.env.get('SENSEI_DAEMON_URL')) ?? 'http://127.0.0.1:7744'

const deadLetter = async ($: EngineInterface, line: string) => {
  const home = await $.env.get('HOME')
  if (home === undefined) throw new Error('HOME is unset; nowhere to spool the event')
  // Appended by a shell, which opens the file O_APPEND: the hook scripts append
  // to the same spool concurrently, and a read-modify-write would clobber them.
  await $.process.run(
    ['/bin/sh', '-c', 'mkdir -p "$(dirname "$1")" && cat >> "$1"', 'sh', `${home}/.sensei/events.jsonl`],
    { stdin: `${line}\n` },
  )
}

const post = async ($: EngineInterface, fields: Fields) => {
  const event = {
    session_id: await $.session.id(),
    assistant_family: 'claude',
    ts: await $.clock.now(),
    ...fields,
  }
  const line = JSON.stringify(event)
  const sent = $.http
    .fetch(`${await daemonUrl($)}/hook/event`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: line,
    })
    .then(r => r.ok, () => false)
  const bound = new AbortController()
  const timedOut = $.clock.sleep(POST_TIMEOUT_MS, { signal: bound.signal }).then(
    () => false,
    () => false,
  )
  const isDelivered = await Promise.race([sent, timedOut])
  bound.abort()
  if (!isDelivered) await deadLetter($, line)
}

export const register: Register = on => {
  on('session.measure', async ($, e, next) => {
    const result = await next(e)
    await post($, {
      hook_event_name: 'SessionMeasure',
      context: e.context,
      rate_limits: e.rateLimits,
      ...(e.cost === undefined ? {} : { cost: e.cost }),
      changed: e.changed,
    })
    return result
  })

  on('turn.complete', async ($, e, next) => {
    const result = await next(e)
    await post($, {
      hook_event_name: 'TurnComplete',
      turn_id: e.turnId,
      ...(e.agentId === undefined ? {} : { agent_id: e.agentId }),
      reason: e.reason,
      duration_ms: e.durationMs,
      ...(e.usage === undefined ? {} : { usage: e.usage }),
    })
    return result
  })
}
