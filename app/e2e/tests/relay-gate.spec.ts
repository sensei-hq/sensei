/**
 * The relay tool gate records its outcome in the run's activity (#312).
 *
 * Runs against the e2e daemon with no dōjō enrolled, so every gate goes
 * unanswered: an unattended hard-block is denied (D28), an interactive one is
 * let through, and both are appended to the run named by `run_id`. An ordinary
 * call is not gated and leaves no record.
 *
 * The scheduler appends a `housekeeping` event to every active run each tick,
 * and `GET /api/runs/{id}` returns only the newest 20 events, so the spec counts
 * only the gate's own events (`detail.via` = `hook_gate`) and reads the run right
 * after each post. Answered replies (approve, decline, unclear) need a dōjō; the
 * daemon's pure and router tests cover them.
 */
import { test, expect } from '../fixtures';
import { DAEMON_URL } from '../helpers';

interface RunEvent {
  kind: string;
  detail: Record<string, unknown>;
}

interface GateAnswer {
  hookSpecificOutput: { permissionDecision: string; permissionDecisionReason: string };
}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${method} ${path}: HTTP ${res.status}: ${await res.text()}`);
  return (await res.json()) as T;
}

async function gate(command: string, runId: string, unattended?: boolean) {
  const body: Record<string, unknown> = {
    session_id: crypto.randomUUID(),
    hook_event_name: 'PreToolUse',
    tool_name: 'Bash',
    tool_input: { command },
    run_id: runId,
  };
  if (unattended !== undefined) body.unattended = unattended;
  return (await call<GateAnswer>('POST', '/hook/gate', body)).hookSpecificOutput;
}

async function gateEvents(runId: string): Promise<RunEvent[]> {
  const { events } = await call<{ events: RunEvent[] }>('GET', `/api/runs/${runId}`);
  return events.filter(e => e.detail?.via === 'hook_gate');
}

test.describe('relay gate: outcomes in the run activity (#312)', () => {
  let runId = '';

  test.beforeAll(async () => {
    // A gate with a uuid session is raised to an enrolled dōjō. The e2e daemon
    // must never reach one, so the spec refuses to run if any is enrolled.
    const memberships = await call<Array<{ enabled?: boolean }>>('GET', '/api/dojo/memberships');
    expect(memberships.filter(m => m.enabled !== false), 'no dōjō enrolled in the e2e DB').toEqual([]);
    const created = await call<{ run: { id: string } }>('POST', '/api/runs', { goal: `relay gate e2e ${Date.now()}` });
    runId = created.run.id;
  });

  test('an unattended deploy is denied and recorded as denied', async () => {
    const answer = await gate('make deploy TARGET=arg-marker-312', runId, true);
    expect(answer.permissionDecision).toBe('deny');
    expect(answer.permissionDecisionReason).toContain('unattended');

    const events = await gateEvents(runId);
    const denied = events.filter(e => e.detail.outcome === 'denied');
    expect(denied).toHaveLength(1);
    expect(denied[0].kind).toBe('flagged');
    expect(denied[0].detail.category).toBe('deploy');
    expect(denied[0].detail.reason).toBe(answer.permissionDecisionReason);
    // The reason is the classifier's own phrase; the command's text never reaches the record.
    expect(JSON.stringify(denied[0].detail)).not.toContain('arg-marker-312');
    expect(denied[0].detail).not.toHaveProperty('tool_input');
  });

  test('an interactive deploy is allowed and recorded as allowed', async () => {
    const answer = await gate('make deploy', runId);
    expect(answer.permissionDecision).toBe('allow');

    const allowed = (await gateEvents(runId)).filter(e => e.detail.outcome === 'allowed');
    expect(allowed).toHaveLength(1);
    expect(allowed[0].kind).toBe('flagged');
    expect(allowed[0].detail.category).toBe('deploy');
  });

  test('an ordinary call is allowed and leaves no gate event', async () => {
    const before = (await gateEvents(runId)).length;
    const answer = await gate('cargo test', runId, true);
    expect(answer.permissionDecision).toBe('allow');
    expect(await gateEvents(runId)).toHaveLength(before);
  });
});
