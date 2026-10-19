// What the metrics screen says when an assistant's conversation-history reading
// is off (#280, decided 2026-10-19): its token, cost and session metrics stop
// updating, and blank metrics alone said nothing about why. Pure.
import type { TranscriptSourceConsent } from '$lib/types.js';

export interface HistoryOffNotice {
  /** The assistants, by the names a person knows them by. */
  assistants: string[];
  text: string;
}

/** `null` when every assistant is read — or when consent could not be read,
 *  since a failed read is not evidence that reading is off. */
export function historyOffNotice(consent: TranscriptSourceConsent[] | null): HistoryOffNotice | null {
  if (!consent) return null;
  const assistants = consent.filter((c) => !c.consented).map((c) => c.label);
  if (assistants.length === 0) return null;
  const names =
    assistants.length === 1
      ? assistants[0]
      : `${assistants.slice(0, -1).join(', ')} and ${assistants[assistants.length - 1]}`;
  return {
    assistants,
    text: `Conversation history reading is off for ${names}, so their token, cost and session metrics are not updating. Turn it on in Settings → Assistants.`,
  };
}
