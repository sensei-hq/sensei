// The SDLC figure words shared by Tickets and Delivery Metrics
// (tracker-sdlc-metrics S8, tracker-delivery-metrics D2).
//
// What must never happen here: a real 0 shown as a reason or "—", a null shown as 0,
// or velocity formatted as a percent.
import { describe, it, expect } from 'vitest';
import { projectSdlcFigures, sprintSdlcFigures, sprintWindowReason } from './sdlc-figures.js';
import { WORDS, projectSdlc, sprintSdlc } from './tracker-fixtures.js';

describe('sprintSdlcFigures', () => {
  it('shows all seven figures in order: six percents, then velocity in points', () => {
    const figures = sprintSdlcFigures(
      sprintSdlc({
        commitment_reliability: 80,
        total_completion: 90,
        scope_change: 40,
        carry_over_rate: 20,
        unplanned_work_ratio: 10,
        velocity: 21,
        points_commitment_reliability: 75,
        reasons: {},
      }),
      WORDS,
    );
    expect(figures.map((f) => [f.key, f.text])).toEqual([
      ['commitment_reliability', '80%'],
      ['total_completion', '90%'],
      ['scope_change', '40%'],
      ['carry_over_rate', '20%'],
      ['unplanned_work_ratio', '10%'],
      ['velocity', '21 pts'],
      ['points_commitment_reliability', '75%'],
    ]);
  });

  it("a null shows its reason's summary, marked as a reason", () => {
    const [first] = sprintSdlcFigures(sprintSdlc(), WORDS);
    expect(first).toMatchObject({ text: 'Ticket history not read yet', reason: true });
  });

  it('a real 0 renders as 0, never as a reason or —', () => {
    const figures = sprintSdlcFigures(
      sprintSdlc({ commitment_reliability: 0, velocity: 0, reasons: {} }),
      WORDS,
    );
    expect(figures.find((f) => f.key === 'commitment_reliability')!.text).toBe('0%');
    expect(figures.find((f) => f.key === 'velocity')!.text).toBe('0 pts');
  });
});

describe('projectSdlcFigures', () => {
  it('a real 0 renders as 0, never as a reason or —', () => {
    const figures = projectSdlcFigures(
      projectSdlc({ predictability: 0, tokens_per_ticket: 0, prompts_per_ticket: 0, reasons: {} }),
      WORDS,
    );
    expect(figures.map((f) => f.text)).toEqual(['0 pts', '0', '0.0']);
  });
});

describe('sprintWindowReason', () => {
  it("names a sprint with no window in Tickets' words, and an unknown code as itself", () => {
    expect(sprintWindowReason('not_started')).toBe('Not started. Usage counts from the sprint’s start.');
    expect(sprintWindowReason('no_dates')).toMatch(/no dates/);
  });
});
