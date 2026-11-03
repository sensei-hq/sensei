// A ticking clock for relative times ("2m ago", "next in 5m") — state, so a
// view re-renders as time passes instead of freezing at the moment it loaded
// (audit 2026-10-28, item 14). Reference-counted: one view leaving does not stop
// it for another.
export class Clock {
  now = $state(Date.now());
  #timer: ReturnType<typeof setInterval> | null = null;
  #viewers = 0;
  #every: number;

  constructor(everyMs = 15_000) {
    this.#every = everyMs;
  }

  start(): void {
    this.#viewers += 1;
    this.now = Date.now();
    if (this.#timer) return;
    this.#timer = setInterval(() => (this.now = Date.now()), this.#every);
  }

  stop(): void {
    this.#viewers = Math.max(0, this.#viewers - 1);
    if (this.#viewers > 0 || !this.#timer) return;
    clearInterval(this.#timer);
    this.#timer = null;
  }
}

/** The app's clock, for every relative time on screen. */
export const clock = new Clock();
