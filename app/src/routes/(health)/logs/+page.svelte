<script lang="ts">
    import type { PageData } from "./$types";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import {
        fmtMs,
        outcomeColor,
        outcomeStatus,
        anonymize,
        formatTime,
        timeOfDay,
        sessionSummary,
        groupByDate,
        moduleLabel,
        isBootstrapTrace,
        actionMeta,
        buildTitle,
        buildBody,
    } from "./helpers.js";
    import { LogsPageState } from "./state.svelte.js";
    import { Eyebrow, Kanji, StatusDot } from "$lib/components";
    import { Button } from "@rokkit/ui";

    let { data }: { data: PageData } = $props();

    const state = new LogsPageState();
    $effect(() => state.setSessions(data.sessions));

    const dateGroups = $derived(groupByDate(state.sessions));

    async function submitToGitHub() {
        if (!state.session) return;
        const url = `https://github.com/sensei-hq/app/issues/new?title=${encodeURIComponent(buildTitle(state.session))}&body=${encodeURIComponent(buildBody(state.session, state.addCtx))}`;
        await openUrl(url);
    }

    async function copyMarkdown() {
        if (!state.session) return;
        await navigator.clipboard.writeText(
            buildBody(state.session, state.addCtx),
        );
        state.setCopied(true);
        setTimeout(() => {
            state.setCopied(false);
        }, 1400);
    }
</script>

<!-- ── Page shell ────────────────────────────────────────────────────────── -->
<div class="h-full flex overflow-hidden text-ink">
    <!-- ── Sidebar ─────────────────────────────────────────────────────────── -->
    <aside
        class="w-[248px] shrink-0 border-r border-paper-edge bg-paper-mute flex flex-col overflow-hidden"
    >
        <div
            class="px-5 pt-6 pb-3.5 border-b border-paper-edge flex items-center gap-2.5 shrink-0"
        >
            <Kanji char="診" size="lg" />
            <Eyebrow>diagnostic logs</Eyebrow>
        </div>

        <div class="flex-1 overflow-y-auto py-2">
            {#each dateGroups as dg (dg.date)}
                {@const open = state.expandedDates.has(dg.date)}

                <!-- Date header -->
                <button
                    class="flex items-center gap-1.5 w-full px-5 pt-2 pb-1 bg-transparent border-none cursor-pointer text-left"
                    onclick={() => state.toggleDate(dg.date)}
                >
                    <span
                        class="text-xs text-ink-soft leading-none inline-block transition-transform duration"
                        class:collapsed={!open}>▾</span
                    >
                    <span
                        class="text-xs font-semibold tracking-wide uppercase text-ink-mute"
                        >{dg.date}</span
                    >
                </button>

                {#if open}
                    {#each dg.sessions as s (s.id)}
                        {@const sel = s.id === state.selectedId}
                        {@const isCurrent = s.id === state.sessions[0]?.id}
                        <button
                            class="session-row block w-full py-2 px-5 pl-9 border-none border-l-2 border-l-transparent cursor-pointer text-left transition-colors duration-fast"
                            class:selected={sel}
                            onclick={() => state.selectSession(s.id)}
                        >
                            {#if isCurrent}
                                <span
                                    class="block text-xs tracking-wide uppercase text-accent mb-0.5"
                                    >current</span
                                >
                            {/if}
                            <div class="flex items-center gap-1.5 mb-0.5">
                                <StatusDot status={outcomeStatus(s.outcome)} size="sm" />
                                <span
                                    class="session-time text-sm font-medium text-ink-mute"
                                    >{timeOfDay(s.started_at)}</span
                                >
                            </div>
                            <div class="ml-3 text-xs text-ink-soft">
                                {moduleLabel(s.module)} · {sessionSummary(s)}
                            </div>
                        </button>
                    {/each}
                {/if}
            {/each}

            {#if state.sessions.length === 0}
                <div class="px-5 py-4 text-xs text-ink-soft">
                    No sessions yet
                </div>
            {/if}
        </div>
    </aside>

    <!-- ── Main panel ──────────────────────────────────────────────────────── -->
    <main class="flex-1 min-w-0 flex flex-col overflow-hidden">
        {#if state.session}
            {@const s = state.session}
            {@const si = s.system_info}
            {@const bt = s.traces.filter(isBootstrapTrace)}
            {@const fixes = bt.filter((t) => t.fix_attempted).length}

            <!-- Session header -->
            <div class="shrink-0 pt-6 px-9 border-b border-paper-edge">
                <div class="flex items-start justify-between mb-4">
                    <div>
                        <div class="mb-1.5"><Eyebrow>{moduleLabel(s.module)}</Eyebrow></div>
                        <h2
                            class="display text-2xl font-normal mb-1.5 tracking-tight"
                        >
                            {formatTime(s.started_at)}
                        </h2>
                        <div
                            class="flex items-center flex-wrap text-xs text-ink-soft"
                        >
                            {#each [anonymize(si.os), si.arch, `${si.ram_gb} GB`, `${si.cpu_cores} cores`, `v${s.app_version}`] as v, i}
                                {#if i > 0}<span class="mx-2 opacity-40">·</span
                                    >{/if}
                                <span>{v}</span>
                            {/each}
                        </div>
                    </div>

                    <Button
                        variant="primary"
                        size="sm"
                        class="shrink-0 mt-1"
                        onclick={() => state.openModal()}
                    >
                        Report this session ↗
                    </Button>
                </div>

                <!-- Stats row -->
                <div class="flex gap-6 pb-4">
                    {#each [{ label: "Total time", value: fmtMs(s.duration_ms) }, { label: "Traces", value: String(bt.length) }, { label: "Auto-fixes", value: String(fixes), color: fixes > 0 ? "var(--warning)" : undefined }, { label: "Outcome", value: s.outcome, color: outcomeColor(s.outcome) }] as stat (stat.label)}
                        <div class="flex flex-col gap-0.5">
                            <Eyebrow>{stat.label}</Eyebrow>
                            <div
                                class="text-sm font-medium"
                                style:color={stat.color ??
                                    "var(--ink)"}
                            >
                                {stat.value}
                            </div>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Column headers -->
            <div
                class="grid grid-cols-[68px_148px_1fr_52px_40px] gap-3 px-9 py-2 border-b border-paper-edge bg-paper-mute shrink-0 text-xs tracking-wide uppercase text-ink-soft"
            >
                <span>action</span>
                <span>step</span>
                <span>command</span>
                <span class="text-right">duration</span>
                <span></span>
            </div>

            <!-- Trace list -->
            <div class="flex-1 overflow-y-auto px-9">
                {#each bt as t (t.id)}
                    {@const am = actionMeta(t.action_type ?? "check")}
                    {@const isOpen = state.expandedTraceId === t.id}
                    {@const hasDetail = !!(t.out || t.err || t.fix_attempted)}

                    <div class="border-b border-paper-edge">
                        <!-- Main row -->
                        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                        <div
                            class="trace-row grid grid-cols-[68px_148px_1fr_52px_40px] gap-3 items-center py-2 {hasDetail
                                ? 'cursor-pointer'
                                : 'cursor-default'}"
                            role={hasDetail ? "button" : "presentation"}
                            tabindex={hasDetail ? 0 : -1}
                            onclick={() => {
                                if (hasDetail) state.toggleTrace(t.id);
                            }}
                            onkeydown={(e) => {
                                if (
                                    hasDetail &&
                                    (e.key === "Enter" || e.key === " ")
                                )
                                    state.toggleTrace(t.id);
                            }}
                        >
                            <span
                                class="text-xs font-semibold tracking-wide px-1.5 py-0.5 rounded-sm whitespace-nowrap justify-self-start"
                                style:color={am.color}
                                style:background={am.bg}
                            >
                                {am.label}
                            </span>
                            <span
                                class="text-xs font-mono text-ink-mute overflow-hidden whitespace-nowrap text-ellipsis"
                                >{t.step ?? ""}</span
                            >
                            <span
                                class="text-xs font-mono text-ink-soft overflow-hidden whitespace-nowrap text-ellipsis"
                                >{anonymize(t.cmd ?? "")}</span
                            >
                            <span
                                class="text-xs text-ink-soft tabular-nums text-right"
                                >{fmtMs(t.ms ?? 0)}</span
                            >
                            <div class="text-center">
                                {#if t.ok}
                                    <span
                                        class="text-success text-sm leading-none"
                                        >✓</span
                                    >
                                {:else if t.fix_ok}
                                    <span
                                        class="text-xs font-semibold tracking-wide text-warning"
                                        >FIXED</span
                                    >
                                {:else}
                                    <span
                                        class="text-accent text-sm leading-none"
                                        >✗</span
                                    >
                                {/if}
                            </div>
                        </div>

                        <!-- Expanded detail -->
                        {#if isOpen && hasDetail}
                            <div
                                class="ml-20 mb-3 px-3.5 py-3 bg-paper-mute rounded-md border border-paper-edge flex flex-col gap-2.5"
                            >
                                {#if t.out}
                                    <div class="flex flex-col gap-1">
                                        <Eyebrow>stdout</Eyebrow>
                                        <pre
                                            class="m-0 text-xs font-mono text-ink-mute leading-normal whitespace-pre-wrap break-all">{anonymize(
                                                t.out,
                                            )}</pre>
                                    </div>
                                {/if}
                                {#if t.err}
                                    <div class="flex flex-col gap-1">
                                        <div
                                            class="detail-block-label err text-xs tracking-wide uppercase"
                                        >
                                            stderr
                                        </div>
                                        <pre
                                            class="err-text m-0 text-xs font-mono leading-normal whitespace-pre-wrap break-all">{t.err}</pre>
                                    </div>
                                {/if}
                                {#if t.fix_attempted}
                                    <div
                                        class="flex flex-col gap-1 pt-2.5 border-t border-paper-edge"
                                    >
                                        <div
                                            class="detail-block-label fix text-xs tracking-wide uppercase"
                                        >
                                            auto-fix attempted
                                        </div>
                                        <pre
                                            class="m-0 text-xs font-mono leading-normal whitespace-pre-wrap break-all"
                                            style:color={t.fix_ok
                                                ? "var(--success)"
                                                : "var(--accent)"}>$ {t.fix_approach ??
                                                ""}  <span
                                                class="text-ink-soft text-xs"
                                                >→ {t.fix_ok
                                                    ? "succeeded"
                                                    : "failed"}</span
                                            ></pre>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                {/each}
                {#if bt.length === 0}
                    <div class="py-10 text-center text-sm text-ink-soft">
                        {#if s.traces.length > 0}
                            Legacy bootstrap session — {s.traces.length} record{s.traces.length === 1 ? "" : "s"} with no detailed step traces.
                        {:else}
                            No steps recorded for this session.
                        {/if}
                    </div>
                {/if}
                <div class="h-8"></div>
            </div>
        {:else}
            <div
                class="flex-1 flex items-center justify-center text-sm text-ink-soft"
            >
                No sessions recorded yet. Run the app to generate logs.
            </div>
        {/if}
    </main>
</div>

<!-- ── Report modal ──────────────────────────────────────────────────────── -->
{#if state.showModal && state.session}
    {@const s = state.session}
    {@const si = s.system_info}
    {@const bt = s.traces.filter(isBootstrapTrace)}
    <div
        class="modal-overlay fixed inset-0 flex items-center justify-center z-30 p-6"
        role="dialog"
        aria-modal="true"
        onclick={(e) => {
            if (e.target === e.currentTarget) state.closeModal();
        }}
        onkeydown={(e) => {
            if (e.key === "Escape") state.closeModal();
        }}
        tabindex="-1"
    >
        <div
            class="modal w-[84vw] max-w-[960px] max-h-[90vh] bg-paper-soft rounded-xl border border-paper-edge flex flex-col"
        >
            <div
                class="flex items-start justify-between pt-4 pb-3 px-7 border-b border-paper-edge shrink-0"
            >
                <div>
                    <div class="display text-lg font-normal mb-1">
                        Report session
                    </div>
                    <div class="text-xs text-ink-soft">
                        {formatTime(s.started_at)} · {bt.length} traces ·
                        {bt.filter((t) => t.fix_attempted).length} auto-fixes
                    </div>
                </div>
                <button
                    class="bg-transparent border-none cursor-pointer text-ink-soft text-xl px-2 py-1 leading-none ml-4 shrink-0"
                    onclick={() => state.closeModal()}>×</button
                >
            </div>

            <div class="flex-1 overflow-auto pt-5 pb-6 px-7 flex gap-5">
                <!-- Issue preview -->
                <div class="flex-1 flex flex-col gap-2 min-h-0 overflow-hidden">
                    <Eyebrow>Issue preview — anonymized</Eyebrow>
                    <div
                        class="bg-paper-mute border border-paper-edge rounded-md px-3 py-2 text-sm text-ink font-medium shrink-0"
                    >
                        {buildTitle(s)}
                    </div>
                    <pre
                        class="flex-1 bg-paper-mute border border-paper-edge rounded-md px-3.5 py-3 font-mono text-xs text-ink-mute leading-loose overflow-auto whitespace-pre-wrap break-words min-h-0 m-0">{buildBody(
                            s,
                            state.addCtx,
                        )}</pre>
                </div>

                <!-- Compose column -->
                <div class="w-[264px] shrink-0 flex flex-col gap-3">
                    <div
                        class="bg-paper-mute border border-paper-edge rounded-md px-3.5 py-3"
                    >
                        <div class="mb-2"><Eyebrow>Included in report</Eyebrow></div>
                        {#each [["Session", formatTime(s.started_at)], ["OS", anonymize(si.os)], ["Arch", si.arch], ["RAM", `${si.ram_gb} GB`], ["Traces", String(bt.length)], ["Fixes", String(bt.filter((t) => t.fix_attempted).length)], ["App", `v${s.app_version}`]] as [k, v]}
                            <div class="flex justify-between text-xs mb-1">
                                <span class="text-ink-soft">{k}</span>
                                <span class="text-ink-mute font-mono text-xs"
                                    >{v}</span
                                >
                            </div>
                        {/each}
                    </div>

                    <div>
                        <div class="mb-2"><Eyebrow>Additional context</Eyebrow></div>
                        <textarea
                            class="ctx-input w-full resize-none font-sans text-sm text-ink bg-paper-mute border border-paper-edge rounded-md px-2.5 py-2 leading-normal"
                            bind:value={state.addCtx}
                            placeholder="What were you doing? Any other details…"
                            rows={4}
                        ></textarea>
                    </div>

                    <p
                        class="privacy-note text-xs text-ink-soft leading-loose m-0"
                    >
                        Paths like <code>/Users/</code> are replaced with
                        <code>~/</code>. No personal data included.
                    </p>

                    <div class="flex flex-col gap-2 mt-auto">
                        <button
                            class="btn-solid w-full text-sm px-4 py-2.5"
                            onclick={submitToGitHub}
                        >
                            Submit to GitHub ↗
                        </button>
                        <button
                            class="outline-btn w-full text-sm px-4 py-2.5 bg-transparent border border-paper-edge text-ink rounded-md cursor-pointer font-sans transition-colors duration"
                            onclick={copyMarkdown}
                        >
                            {state.copied ? "Copied ✓" : "Copy markdown"}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
{/if}

<style>
    /* Date chevron collapsed rotation */
    .collapsed {
        transform: rotate(-90deg);
    }

    /* Session row hover/selected states */
    .session-row:hover {
        background: var(--paper-mute);
    }
    .session-row.selected {
        background: var(--paper-soft);
        border-left-color: var(--accent);
    }
    .session-row.selected .session-time {
        color: var(--ink);
    }

    /* Detail label variants */
    .detail-block-label.err {
        color: var(--accent);
    }
    .detail-block-label.fix {
        color: var(--warning);
    }

    /* Error text override */
    .err-text {
        color: var(--accent) !important;
    }

    /* Input focus ring */
    .ctx-input:focus {
        outline: none;
        border-color: var(--ink-soft);
    }

    /* Code snippets in privacy note */
    .privacy-note code {
        font-family: var(--font-mono);
        font-size: 10.5px;
        background: var(--paper-mute);
        padding: 1px 8px;
        border-radius: 3px;
    }

    /* Outline button hover */
    .outline-btn:hover {
        background: var(--paper-mute);
    }

    /* Modal overlay backdrop */
    .modal-overlay {
        background: rgba(0, 0, 0, 0.3);
    }

    /* Modal drop shadow */
    .modal {
        box-shadow: 0 24px 60px rgba(0, 0, 0, 0.18);
    }
</style>
