// What ScreenState's error branch says (pure). Kept beside the component so the
// template stays a pure render: a screen that has a `notice` from its state
// module (`noticeFromApiError`) passes it whole; one that only has an error string
// gets the generic wording, which names the daemon because that is the one thing
// every sensei screen depends on.

export interface ScreenNotice {
  tone?: 'fault' | 'refusal' | 'action' | 'normal';
  eyebrow?: string;
  title: string;
  detail?: string;
  remedy?: string;
  code?: string;
}

const DAEMON_REMEDY = 'Check that the sensei daemon is running, then retry. Health shows which part is down.';

export function screenErrorNotice(error: string | null | undefined, notice?: ScreenNotice): Required<Pick<ScreenNotice, 'tone' | 'eyebrow' | 'title'>> & ScreenNotice {
  if (notice) return { tone: 'fault', eyebrow: "couldn't load", ...notice };
  return {
    tone: 'fault',
    eyebrow: "couldn't load",
    title: "Couldn't load this",
    detail: error || undefined,
    remedy: error ? undefined : DAEMON_REMEDY,
  };
}
