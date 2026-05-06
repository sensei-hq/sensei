/**
 * Pure helper functions for the Diagnostic Logs page.
 * Extracted so they can be unit-tested independently of the Svelte component.
 */
import type { LogSession, BootstrapTrace } from '$lib/types.js';

// ── Action badge ──────────────────────────────────────────────────────────

export const ACTION_META: Record<string, { label: string; color: string; bg: string }> = {
    check:    { label: 'CHECK',    color: 'oklch(var(--color-surface-z6) / 1)',   bg: 'oklch(var(--color-surface-z3) / 1)' },
    resolve:  { label: 'RESOLVE',  color: 'oklch(var(--color-warning-z5) / 1)',   bg: 'oklch(var(--color-warning-z5) / 0.10)' },
    instruct: { label: 'INSTRUCT', color: 'oklch(var(--color-primary-z5) / 1)',   bg: 'oklch(var(--color-primary-z5) / 0.08)' },
};

export function actionMeta(type: string): { label: string; color: string; bg: string } {
    return ACTION_META[type] ?? ACTION_META.check;
}

// ── Module metadata ───────────────────────────────────────────────────────

export const MODULE_META: Record<string, { label: string; kanji: string }> = {
    bootstrap: { label: 'Bootstrap',    kanji: '健' },
    wizard:    { label: 'Setup wizard', kanji: '導' },
    projects:  { label: 'Projects',     kanji: '組' },
};

export function moduleLabel(mod: string): string {
    return MODULE_META[mod]?.label ?? mod;
}

export function moduleKanji(mod: string): string {
    return MODULE_META[mod]?.kanji ?? '◆';
}

// ── Formatting ────────────────────────────────────────────────────────────

export function fmtMs(ms: number): string {
    return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
}

export function outcomeColor(o: string): string {
    return o === 'success' ? 'oklch(var(--color-success-z5) / 1)' : o === 'partial' ? 'oklch(var(--color-warning-z5) / 1)' : 'oklch(var(--color-primary-z5) / 1)';
}

export const anonymize = (s: string): string => s.replace(/\/Users\/[^/]+\//g, '~/');

export function formatTime(ts: string): string {
    const d = new Date(ts);
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    const dDay = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    const timeStr = d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
    if (dDay.getTime() === today.getTime())     return `Today · ${timeStr}`;
    if (dDay.getTime() === yesterday.getTime()) return `Yesterday · ${timeStr}`;
    return `${d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })} · ${timeStr}`;
}

/** Returns just the date label portion: "Today", "Yesterday", "Apr 29". */
export function dateKey(ts: string): string {
    return formatTime(ts).split(' · ')[0];
}

/** Returns just the time portion: "10:42". */
export function timeOfDay(ts: string): string {
    return formatTime(ts).split(' · ')[1] ?? '';
}

/** One-line summary for the sidebar: "3.2s · 12 steps · 2 fixes". */
export function sessionSummary(s: LogSession): string {
    const fixes = s.traces.filter(t => isBootstrapTrace(t) && (t as BootstrapTrace).fix_attempted).length;
    const parts = [fmtMs(s.duration_ms), `${s.traces.length} steps`];
    if (fixes > 0) parts.push(`${fixes} fix${fixes > 1 ? 'es' : ''}`);
    return parts.join(' · ');
}

// ── Grouping ──────────────────────────────────────────────────────────────

export interface DateGroup { date: string; sessions: LogSession[] }

/**
 * Groups sessions by date label (newest first), flat list within each date.
 * Sessions are sorted reverse-chronologically before grouping.
 */
export function groupByDate(sessions: LogSession[]): DateGroup[] {
    const sorted = [...sessions].sort(
        (a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime()
    );

    const dateOrder: string[] = [];
    const dateMap: Record<string, LogSession[]> = {};

    for (const s of sorted) {
        const dk = dateKey(s.started_at);
        if (!dateMap[dk]) { dateMap[dk] = []; dateOrder.push(dk); }
        dateMap[dk].push(s);
    }

    return dateOrder.map(dk => ({ date: dk, sessions: dateMap[dk] }));
}

// ── Type guard ────────────────────────────────────────────────────────────

export function isBootstrapTrace(t: unknown): t is BootstrapTrace {
    return typeof t === 'object' && t !== null && 'action_type' in t;
}

// ── Issue builder ─────────────────────────────────────────────────────────

export function buildTitle(s: LogSession): string {
    return `Bootstrap diagnostic — ${anonymize(s.system_info.os)} · ${s.system_info.arch} · v${s.app_version}`;
}

export function buildBody(s: LogSession, ctx: string): string {
    const bt = s.traces.filter(isBootstrapTrace);
    const rows = bt.map(t =>
        `| ${t.step} | ${t.action_type} | \`${anonymize(t.cmd ?? '')}\` | ${fmtMs(t.ms ?? 0)} | ${t.ok ? '✓' : t.fix_ok ? '✓ fixed' : '✗'} |`
    ).join('\n');

    const ctxSection = ctx ? `\n\n## Additional context\n\n${ctx}` : '';

    return `## System info
- OS: ${anonymize(s.system_info.os)}
- Architecture: ${s.system_info.arch}
- RAM: ${s.system_info.ram_gb} GB · ${s.system_info.cpu_cores} cores
- App version: v${s.app_version}
- Session: ${formatTime(s.started_at)}

## Bootstrap trace

| Step | Type | Command | Duration | Result |
|------|------|---------|----------|--------|
${rows}${ctxSection}`;
}
