// The sensei daemon, as the plugin's mods modules reach it: its URL, a bounded
// request, and the capture spool a missed event falls back to. Mirrors the
// shell hooks' _lib.sh so every capture path agrees.
import type { EngineInterface } from 'claude-code'

// A request's own bound, so a hung daemon cannot hold a hook to its budget.
const REQUEST_TIMEOUT_MS = 2000

export type Fields = Record<string, unknown>

async function daemonUrl($: EngineInterface) {
  return (await $.env.get('SENSEI_DAEMON_URL')) ?? 'http://127.0.0.1:7744'
}

/**
 * POSTs `body` as JSON to the daemon's `path`, resolving the response text on a
 * 2xx and `undefined` on any failure: unreachable, non-2xx, or slower than the
 * bound.
 */
export async function request($: EngineInterface, path: string, body: string) {
  const sent = $.http
    .fetch(`${await daemonUrl($)}${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    })
    .then(
      r => (r.ok ? r.text : undefined),
      () => undefined,
    )
  const bound = new AbortController()
  const timedOut = $.clock.sleep(REQUEST_TIMEOUT_MS, { signal: bound.signal }).then(
    () => undefined,
    () => undefined,
  )
  const answer = await Promise.race([sent, timedOut])
  bound.abort()
  return answer
}

async function deadLetter($: EngineInterface, line: string) {
  const home = await $.env.get('HOME')
  if (home === undefined) throw new Error('HOME is unset; nowhere to spool the event')
  // Appended by a shell, which opens the file O_APPEND: the hook scripts append
  // to the same spool concurrently, and a read-modify-write would clobber them.
  await $.process.run(
    ['/bin/sh', '-c', 'mkdir -p "$(dirname "$1")" && cat >> "$1"', 'sh', `${home}/.sensei/events.jsonl`],
    { stdin: `${line}\n` },
  )
}

/**
 * Captures one event: POSTs it to `/hook/event`, and on any failure dead-letters
 * the line to `~/.sensei/events.jsonl`, which the daemon's capture_drain imports.
 * Each line carries its own `ts`, so a drained event keeps the time it happened.
 */
export async function capture($: EngineInterface, fields: Fields) {
  const line = JSON.stringify({
    session_id: await $.session.id(),
    assistant_family: 'claude',
    ts: await $.clock.now(),
    ...fields,
  })
  if ((await request($, '/hook/event', line)) === undefined) await deadLetter($, line)
}
