// A Settings section's one read, as state (frontend guidelines §2): Providers and
// Libraries each read once on mount, and used to keep `loading` / `error` as
// component locals. A failed read is said through the shared Notice
// (packages/ui/notice.js) — never an empty list.
import { failureNotice } from '@sensei/ui';

export class SectionRead {
  loading = $state(true);
  error = $state<string | null>(null);
  #title: string;

  /** `title` names what failed ("Couldn't load routers"). */
  constructor(title: string) {
    this.#title = title;
  }

  get notice() {
    return failureNotice(this.#title, this.error);
  }

  async run(read: () => Promise<unknown>): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      await read();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }
}
