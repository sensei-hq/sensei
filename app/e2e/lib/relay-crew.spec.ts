// The fake crew driver (#426): the lane HTTP API as the journey plays it — the
// TypeScript twin of scripts/relay-queue-smoke.sh. Driven with a fake fetch, so each
// call's path, token and body are pinned without a daemon.
import { describe, it, expect, vi } from 'vitest';
import { RelayCrew, RelayCallError, RelayLane } from './relay-crew';

function fakeFetch(answers: Record<string, { status: number; body: unknown }>) {
  const calls: Array<{ method: string; path: string; auth: string | null; body: unknown }> = [];
  const fetch = vi.fn(async (url: string, init: RequestInit = {}) => {
    const path = url.replace('http://daemon', '');
    const headers = (init.headers ?? {}) as Record<string, string>;
    calls.push({
      method: init.method ?? 'GET',
      path,
      auth: headers.Authorization ?? null,
      body: init.body ? JSON.parse(init.body as string) : null,
    });
    const answer = answers[`${init.method ?? 'GET'} ${path.split('?')[0]}`] ?? { status: 200, body: {} };
    return new Response(JSON.stringify(answer.body), { status: answer.status });
  });
  return { fetch: fetch as unknown as typeof globalThis.fetch, calls };
}

const registered = { lane_id: 'lane-1', lane_token: 'tok-1', crew_id: 'crew-1', lane_title: 'worker-a', suggested_name: 'worker-a' };

describe('RelayCrew · register', () => {
  it('registers with no token and hands back a lane that signs every call with its own', async () => {
    const f = fakeFetch({ 'POST /api/relay/register': { status: 200, body: registered } });
    const crew = new RelayCrew('http://daemon', f.fetch);
    const lane = await crew.register({ family: 'e2e', suggested_name: 'worker-a', crew_id: 'crew-1' });
    expect(f.calls[0]).toMatchObject({ method: 'POST', path: '/api/relay/register', auth: null, body: { family: 'e2e', suggested_name: 'worker-a', crew_id: 'crew-1' } });
    expect([lane.id, lane.crewId, lane.name]).toEqual(['lane-1', 'crew-1', 'worker-a']);
    await lane.heartbeat();
    expect(f.calls[1]).toMatchObject({ path: '/api/relay/heartbeat', auth: 'Bearer tok-1', body: { lane_id: 'lane-1' } });
  });
});

describe('RelayLane · the loop', () => {
  async function lane(answers: Record<string, { status: number; body: unknown }> = {}) {
    const f = fakeFetch({ 'POST /api/relay/register': { status: 200, body: registered }, ...answers });
    const l = await new RelayCrew('http://daemon', f.fetch).register({ family: 'e2e' });
    f.calls.length = 0;
    return { l, calls: f.calls };
  }

  it('claim_next, checkpoint with steps, report, claim_review, review, unregister: each names the lane', async () => {
    const { l, calls } = await lane({
      'POST /api/relay/tasks/claim_next': { status: 200, body: { claim_type: 'task', task: { id: 't1' } } },
    });
    expect((await l.claimNext()).task?.id).toBe('t1');
    await l.checkpoint('t1', { current_step: 'tests', steps: [{ key: 'tests', title: 'Red tests', state: 'open' }] });
    await l.report('t1', { phase: 'impl', round: 1, summary: { changed: 'x' }, branch: 'lane/t1' });
    await l.claimReview('t0');
    await l.review('t0', 'approved', 'Sound.');
    await l.unregister();
    expect(calls.map((c) => c.path)).toEqual([
      '/api/relay/tasks/claim_next',
      '/api/relay/tasks/t1/checkpoint',
      '/api/relay/tasks/t1/report',
      '/api/relay/tasks/t0/claim_review',
      '/api/relay/tasks/t0/review',
      '/api/relay/unregister',
    ]);
    for (const c of calls) {
      expect(c.auth).toBe('Bearer tok-1');
      expect((c.body as { lane_id: string }).lane_id).toBe('lane-1');
    }
    expect(calls[1].body).toMatchObject({ current_step: 'tests', steps: [{ key: 'tests', state: 'open' }] });
    expect(calls[4].body).toMatchObject({ verdict: 'approved', summary: 'Sound.' });
  });

  it('a plan report carries the plan as a json block in report_md', async () => {
    const { l, calls } = await lane();
    const plan = { tasks: [{ key: 't1', title: 'First', phase: 'impl', owned_paths: ['a'], acceptance: ['a exists'], deps: [] }] };
    await l.reportPlan('t0', plan, 'Planned one task.');
    const body = calls[0].body as { phase: string; report_md: string; summary: { changed: string } };
    expect(body.phase).toBe('planning');
    expect(body.summary.changed).toBe('Planned one task.');
    expect(body.report_md).toContain('```json');
    expect(JSON.parse(body.report_md.split('```json')[1].split('```')[0])).toEqual(plan);
  });

  it('asks the person a decision on a task', async () => {
    const { l, calls } = await lane({ 'POST /api/relay/decisions': { status: 200, body: { decision_id: 'd1' } } });
    const id = await l.ask({ task_id: 't0', question: 'One crate or two?', options: [{ id: 'one', label: 'One' }, { id: 'two', label: 'Two' }] });
    expect(id).toBe('d1');
    expect(calls[0].body).toMatchObject({ lane_id: 'lane-1', asked_of: 'person', task_id: 't0', question: 'One crate or two?' });
  });

  it('a refusal throws with its status and the daemon’s body; attempt() returns it instead', async () => {
    const { l } = await lane({
      'POST /api/relay/tasks/t2/report': { status: 409, body: { error: 'relay_upstream_deps_unmet', message: 'deps unmet' } },
    });
    const err = await l.report('t2', { phase: 'impl', round: 1, summary: { changed: 'y' } }).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(RelayCallError);
    expect((err as RelayCallError).status).toBe(409);
    expect((err as RelayCallError).body).toMatchObject({ error: 'relay_upstream_deps_unmet' });
    const tried = await RelayLane.attempt(() => l.report('t2', { phase: 'impl', round: 1, summary: { changed: 'y' } }));
    expect(tried).toMatchObject({ ok: false, status: 409, error: 'relay_upstream_deps_unmet' });
  });
});
