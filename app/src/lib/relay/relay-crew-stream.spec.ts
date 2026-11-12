import { describe, it, expect } from 'vitest';
import type { RelayStreamEvent } from '$lib/relay-api.js';
import { crewEventReReads } from './relay-crew-stream.js';

// Queue cutover (phase 4): the crew screen subscribes like the task route
// does. The dispatcher is pure: which event re-reads which read. A run
// event moves the band and the queue (items planned); a lane or task event
// moves Now (lanes, Needs you, the review queue); a step is the task
// route's business.

const ev = (entity: string, data: RelayStreamEvent['data'] = {}): RelayStreamEvent => ({
  action: 'update',
  entity,
  data,
});

describe('crewEventReReads', () => {
  it('a run event re-reads now and the queue', () => {
    for (const op of ['created', 'activated', 'completed']) {
      expect(crewEventReReads(ev('run', { op, run_id: 'r', crew_id: 'c' }))).toEqual({ now: true, queue: true });
    }
  });

  it('a lane or task event re-reads now only', () => {
    expect(crewEventReReads(ev('lane', { op: 'registered', lane_id: 'l', crew_id: 'c' }))).toEqual({ now: true, queue: false });
    for (const op of ['claim', 'claim_review', 'release', 'report', 'review', 'unpack', 'gate', 'message', 'answer']) {
      expect(crewEventReReads(ev('task', { op, task_id: 't' }))).toEqual({ now: true, queue: false });
    }
    expect(crewEventReReads(ev('relay', { op: 'unregister' }))).toEqual({ now: true, queue: false });
  });

  it('a step event re-reads nothing here', () => {
    expect(crewEventReReads(ev('step', { task_id: 't' }))).toEqual({ now: false, queue: false });
    expect(crewEventReReads(ev('workspace'))).toEqual({ now: false, queue: false });
  });
});
