// Consent to read an assistant's conversation history (#218).
//
// A SEPARATE consent from configuring the assistant. Configuring writes hooks
// into the assistant's settings; reading transcripts copies whole past
// conversations — code, pasted credentials, client names — into sensei's
// database. The daemon now reads only the sources consented to here, and every
// source starts off.
//
// The switch shows what the daemon STORED. A write that fails leaves it where it
// was: a yes on screen that the daemon never recorded is the one lie this
// control cannot tell.

import type { ApiResult } from '$lib/api.js';
import type { TranscriptSourceConsent } from '$lib/types.js';

export interface TranscriptConsentApi {
  tryGetTranscriptConsent: () => Promise<ApiResult<TranscriptSourceConsent[]>>;
  trySetTranscriptConsent: (
    source: string,
    consented: boolean,
  ) => Promise<ApiResult<{ ok: boolean; source: string; consented: boolean }>>;
}

function failureText(error: { status: number; message: string }): string {
  return error.status === 0
    ? `Could not reach the daemon — ${error.message}`
    : `The daemon returned ${error.status} ${error.message}`;
}

export class TranscriptConsent {
  #api: TranscriptConsentApi;

  sources = $state<TranscriptSourceConsent[]>([]);
  failure = $state<string | null>(null);
  saving = $state<string | null>(null);

  constructor(api: TranscriptConsentApi) {
    this.#api = api;
  }

  get explanation(): string {
    return 'Reads past conversations from disk and stores them in sensei’s local database. Off until you turn it on. Configuring an assistant does not turn this on.';
  }

  async load() {
    this.failure = null;
    const res = await this.#api.tryGetTranscriptConsent();
    if (res.ok) this.sources = res.data;
    else this.failure = failureText(res.error);
  }

  async toggle(source: string) {
    const current = this.sources.find((s) => s.source === source);
    if (!current) return;
    this.saving = source;
    this.failure = null;
    const res = await this.#api.trySetTranscriptConsent(source, !current.consented);
    this.saving = null;
    if (!res.ok) {
      this.failure = failureText(res.error);
      return;
    }
    this.sources = this.sources.map((s) =>
      s.source === source ? { ...s, consented: res.data.consented } : s,
    );
  }
}
