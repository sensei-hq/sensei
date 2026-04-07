import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

function ftrBadge(ftr: number | null): { badge: string; badgeCls: string } {
  if (ftr == null) return { badge: 'No data',  badgeCls: 'bg-surface-z2 text-surface-z5 border-surface-z3' };
  const pct = Math.round(Number(ftr) * 100);
  if (pct >= 85) return { badge: `${pct}% FTR`, badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' };
  if (pct >= 65) return { badge: `${pct}% FTR`, badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' };
  return           { badge: `${pct}% FTR`, badgeCls: 'bg-error-z1 text-error-z7 border-error-z3'   };
}

export const GET: RequestHandler = async ({ fetch }) => {
  const [accountStatsRes, ftrDistRes, modelUsageRes] = await Promise.all([
    fetch('/data/platform/account_stats'),
    fetch('/data/platform/ftr_distribution'),
    fetch('/data/platform/tool_usage'),
  ]);

  const { data: accountStats } = await accountStatsRes.json();
  const { data: ftrDist }      = await ftrDistRes.json();
  const { data: modelUsage }   = await modelUsageRes.json();

  type AccountStat = { account_slug: string; account_type: string; sessions_30d: number; avg_ftr: number | null; avg_token_cost: number | null };
  const accounts    = (accountStats ?? []) as AccountStat[];
  const active      = accounts.filter(a => Number(a.sessions_30d) > 0);
  const totalSess   = accounts.reduce((sum, a) => sum + Number(a.sessions_30d), 0);
  const ftrValues   = accounts.filter(a => a.avg_ftr != null).map(a => Number(a.avg_ftr));
  const platformFtr = ftrValues.length
    ? Math.round(ftrValues.reduce((s, v) => s + v, 0) / ftrValues.length * 100)
    : null;
  const costValues = accounts.filter(a => a.avg_token_cost != null).map(a => Number(a.avg_token_cost));
  const avgCost    = costValues.length
    ? costValues.reduce((s, v) => s + v, 0) / costValues.length
    : null;

  const stats = [
    { label: 'Active Tenants',   value: String(active.length)                               },
    { label: 'Sessions (30d)',   value: totalSess.toLocaleString()                           },
    ...(platformFtr != null ? [{ label: 'Platform Avg FTR', value: `${platformFtr}%`      }] : []),
    ...(avgCost     != null ? [{ label: 'Avg Token Cost',   value: `$${avgCost.toFixed(4)}`}] : []),
  ];

  // FTR histogram — 10 equal-width buckets 0–1
  type FtrBucket = { bucket: number; bucket_min: number; bucket_max: number; session_count: number };
  const buckets  = (ftrDist ?? []) as FtrBucket[];
  const maxCount = Math.max(...buckets.map(b => b.session_count), 1);
  const ftrBuckets = buckets.map(b => {
    const pctMin   = Math.round(b.bucket_min * 100);
    const pctMax   = Math.round(b.bucket_max * 100);
    const height   = Math.round((b.session_count / maxCount) * 100);
    const colorCls = pctMax <= 40 ? 'bg-error-z5' : pctMax <= 70 ? 'bg-warning-z5' : 'bg-success-z5';
    return { label: `${pctMin}–${pctMax}%`, height, colorCls, count: b.session_count };
  });

  // Model call share (replaces hardcoded MCP tool list)
  type ModelStat = { model: string; total_calls: number };
  const models     = (modelUsage ?? []) as ModelStat[];
  const totalCalls = models.reduce((s, m) => s + Number(m.total_calls), 0);
  const mcpTools   = models.map(m => ({
    name: m.model ?? 'unknown',
    pct:  totalCalls > 0 ? Math.round(Number(m.total_calls) / totalCalls * 100) : 0,
  }));

  const tenants = accounts.slice(0, 10).map(a => ({
    name:     a.account_slug,
    desc:     a.account_type,
    sessions: Number(a.sessions_30d),
    cost:     a.avg_token_cost != null ? `$${Number(a.avg_token_cost).toFixed(4)}` : '—',
    ftr:      a.avg_ftr != null ? Math.round(Number(a.avg_ftr) * 100) : null,
    ...ftrBadge(a.avg_ftr),
  }));

  return json({ stats, ftrBuckets, mcpTools, tenants });
};
