/** One recommendation as displayed in the triage columns — the wire fields
 *  the UI actually reads, plus the project label we JOIN on the client
 *  because the per-project endpoint doesn't carry it. */
export interface TriageRec {
  id: string;
  projectId: string;
  projectName: string;
  title: string;
  why: string;
  urgency: 'high' | 'medium' | 'low';
  status: 'pending' | 'accepted' | 'dismissed' | 'superseded';
  actedAt: string | null;
}

export interface TriageBuckets {
  now: TriageRec[];
  soon: TriageRec[];
  settled: TriageRec[];
}

/** Pure: split recs into Now / Soon / Settled per the mockup.
 *  - Now = pending + high urgency (act this week)
 *  - Soon = pending + medium urgency (worth a look)
 *  - Settled = accepted + dismissed decisions (last 30 days, cap for
 *    read-noise; the UI can offer "show more" later)
 *  Order: Now/Soon by title asc (deterministic); Settled by actedAt desc. */
export function triageBuckets(recs: TriageRec[]): TriageBuckets {
  const now: TriageRec[] = [];
  const soon: TriageRec[] = [];
  const settled: TriageRec[] = [];

  for (const r of recs) {
    if (r.status === 'pending' && r.urgency === 'high')   now.push(r);
    else if (r.status === 'pending' && r.urgency === 'medium') soon.push(r);
    else if (r.status === 'accepted' || r.status === 'dismissed') settled.push(r);
    // low-urgency pending + superseded drop off — they're archive-noise
    // for the triage view.
  }

  now.sort((a, b) => a.title.localeCompare(b.title));
  soon.sort((a, b) => a.title.localeCompare(b.title));
  settled.sort((a, b) => (b.actedAt ?? '').localeCompare(a.actedAt ?? ''));
  return { now, soon, settled: settled.slice(0, 20) };
}
