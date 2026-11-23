// The relay person routes the journeys call directly (no lane token), and the clean
// slate a crew journey starts from: earlier specs leave crews and runs open on the
// shared e2e broker.
import { DAEMON_URL } from '../helpers';

/** A person route on the daemon (no lane token). A refusal throws with its body. */
export async function person<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${method} ${path}: HTTP ${res.status}: ${await res.text()}`);
  return (await res.json()) as T;
}

/** End every open run and crew, so a journey starts with no crew and counts only its own
 *  lanes. Backlog and triage leftovers stay: journeys pick their own rows by title. */
export async function cleanSlate(): Promise<void> {
  const runs = await person<{ active: Array<{ id: string }> }>('GET', '/relay/runs');
  for (const run of runs.active) await person('POST', `/relay/runs/${run.id}/cancel`, {});
  const now = await person<{ crews: Array<{ id: string }> }>('GET', '/relay/now');
  for (const crew of now.crews) await person('POST', `/relay/crews/${crew.id}/close`, {});
}
