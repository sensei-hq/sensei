// packages/ui/types.ts — the shapes the shared family's props share.

/** A next step: a button, or a link when `href` is set. */
export interface NoticeAction {
  label: string;
  onclick?: () => void;
  /** A link instead of a button — "Open Health", "See plans". */
  href?: string;
  /** A page outside the app: opens in the browser, never the webview. */
  external?: boolean;
  /** In flight: the button is disabled so it cannot be pressed twice. */
  busy?: boolean;
  /** The `data-action` hook (default `primary`) — e.g. `retry`. */
  name?: string;
}

export type NoticeTone = 'fault' | 'refusal' | 'action' | 'normal';
