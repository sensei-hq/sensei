// The relay selectors (#421; design 2026-11-12 §2b): pure, from the read to what a
// screen shows. The four stats on Now (CONTRACT §1c-0) are counted from the read the
// daemon gave, never guessed; a number it did not give reads as such (D83).
import { describe, it, expect } from 'vitest';
import { toStats } from './relay-selectors.js';
import { coordinatorLane, decision, floatingReview, lane, minutesAgo, now } from './fixtures.js';

describe('toStats · the four stats, counted from the read', () => {
  const busy = () =>
    now({
      // the coordinator's task is in review (its report floats) — not working;
      // L1 works; L3 lost its lease; L4 is idle with nothing claimed.
      lanes: [
        coordinatorLane(),
        lane(),
        lane({ id: 'l3', code: 'L3', title: 'Billing', lease_state: 'expired', task: null }),
        lane({ id: 'l4', code: 'L4', title: 'Sync hardening', task: null }),
      ],
      floating_reviews: [
        floatingReview({ reported_at: minutesAgo(26) }),
        floatingReview({ task_id: 't-libs', title: 'Libraries page', reported_at: minutesAgo(52) }),
      ],
      decisions: [decision()],
    });

  it('Working counts lanes on a task or holding a review', () => {
    expect(toStats(busy(), Date.now())?.working).toBe(1);
    const reviewing = now({ lanes: [lane({ task: null, reviewing: { id: 'r1', title: 'Notice polish', phase: 'impl', round: 1, claimed_at: minutesAgo(3) } })] });
    expect(toStats(reviewing, Date.now())?.working).toBe(1);
  });

  it('Waiting counts floating reviews and idle lanes, oldest by the oldest floating report', () => {
    expect(toStats(busy(), Date.now())?.waiting).toEqual({ count: 3, oldestMin: 52 });
    expect(toStats(now(), Date.now())?.waiting).toEqual({ count: 0, oldestMin: null });
  });

  it('Blocked counts open decisions and lanes whose lease is lost', () => {
    expect(toStats(busy(), Date.now())?.blocked).toBe(2);
  });

  it('Completed today is the list, with the first-pass line', () => {
    expect(toStats(now(), Date.now())).toMatchObject({ done: 2, firstPass: '1 of 5 first-pass' });
    const quiet = now({ today: { completed: 0, reviewed: null, first_pass_approved: null }, completed_today: [] });
    expect(toStats(quiet, Date.now())).toMatchObject({ done: 0, firstPass: 'first pass not reported' });
  });

  it('no read, no stats', () => {
    expect(toStats(null, Date.now())).toBeNull();
  });
});

