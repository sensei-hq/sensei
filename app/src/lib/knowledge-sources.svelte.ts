/**
 * Knowledge-sources state — drives the Federation "Knowledge sources" screen.
 * The daemon owns federation; this module mirrors its URL-validation rule on
 * the client so an invalid endpoint is rejected before the round-trip, and
 * keeps the add-form / list state for the page (a pure template).
 *
 * The api client is passed into each method as the seam, so the state is
 * testable with a hand-rolled mock and never reaches for a global.
 */

import type { SenseiApi } from './api.js';
import type { KnowledgeSource } from './types.js';

/** Hosts allowed to use plain http — everything else must be https. Mirrors
 *  the daemon's loopback exception. */
const LOOPBACK_HOSTS = new Set(['127.0.0.1', '::1', 'localhost']);

/**
 * Validate a federation source URL. Returns null when valid, else a short
 * error message. Mirrors the daemon, which rejects non-https URLs unless the
 * host is loopback (127.0.0.1, ::1, localhost). Pure — no side effects.
 */
export function validateSourceUrl(url: string): string | null {
  if (!url.trim()) return 'url required';

  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return 'invalid url';
  }

  if (parsed.protocol === 'https:') return null;
  // URL.hostname strips brackets from IPv6, so ::1 compares cleanly.
  if (parsed.protocol === 'http:' && LOOPBACK_HOSTS.has(parsed.hostname)) return null;

  return 'non-loopback url must be https';
}

export class KnowledgeSourcesState {
  sources = $state<KnowledgeSource[]>([]);
  loading = $state(true);
  error = $state<string | null>(null);

  // Add-form fields.
  url = $state('');
  key = $state('');
  name = $state('');
  addError = $state<string | null>(null);
  adding = $state(false);

  async load(api: SenseiApi): Promise<void> {
    this.loading = true;
    this.error = null;
    const { sources } = await api.listKnowledgeSources();
    this.sources = sources;
    this.loading = false;
  }

  async add(api: SenseiApi): Promise<void> {
    const invalid = validateSourceUrl(this.url);
    if (invalid) {
      this.addError = invalid;
      return;
    }

    this.adding = true;
    this.addError = null;
    const body = {
      url: this.url,
      ...(this.key ? { key: this.key } : {}),
      ...(this.name ? { name: this.name } : {}),
    };
    const res = await api.createKnowledgeSource(body);
    this.adding = false;

    if (res.ok) {
      this.url = '';
      this.key = '';
      this.name = '';
      await this.load(api);
    } else {
      this.addError = res.error.message;
    }
  }

  async remove(api: SenseiApi, id: string): Promise<void> {
    const res = await api.deleteKnowledgeSource(id);
    if (res.ok) await this.load(api);
    else this.error = res.error.message;
  }

  async sync(api: SenseiApi, id: string): Promise<void> {
    const res = await api.syncKnowledgeSource(id);
    if (res.ok) await this.load(api);
    else this.error = res.error.message;
  }
}
