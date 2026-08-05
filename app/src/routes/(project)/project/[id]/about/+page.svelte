<script lang="ts">
    import { PageHeader, Kanji } from '$lib/components';
    import { Button, Select } from '@rokkit/ui';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { page } from '$app/state';
    import {
        BindingAction,
        confirmedChip,
        inferredChip,
        resolveBinding,
        type ConfirmedBinding,
        type InferredBinding,
    } from './about-binding-state.svelte.js';
    import {
        MATURITY_OPTIONS,
        ProjectMetadataForm,
        saveStatusLabel,
        type EditableProject,
    } from './about-metadata-state.svelte.js';

    let { data } = $props();
    let p = $derived(data.project);
    const projectId = $derived(page.params.id ?? '');

    // R3 project→Dōjō binding. The view model (confirmed | inferred | empty) is
    // resolved in the state module; the confirm action + its client-side
    // confirmed override live in `BindingAction` so this stays a template.
    const bindAction = new BindingAction();
    const binding = $derived(
        resolveBinding(p, data.suggestion, data.memberships),
    );
    // A successful confirm swaps the inferred row for the confirmed view
    // client-side (the loaded data is stale until the next navigation).
    const bindingView = $derived(bindAction.confirmed ?? binding);

    async function confirmBinding(inferred: InferredBinding) {
        await bindAction.confirm(senseiApi(appState.port), projectId, inferred);
    }

    // Editable identity metadata (status/client/goal/description). The form
    // model + save controller live in the state module so this stays a
    // template. Auto-saves on change with an idle/saving/saved/error status,
    // mirroring the settings-general form. `persist` fires only from the field
    // handlers (never during hydrate), so no "initial load" guard is needed.
    const form = new ProjectMetadataForm();
    // Mutable copy of the readonly enum list for rokkit Select's `items` prop.
    const maturityItems = [...MATURITY_OPTIONS];
    let saveTimer: ReturnType<typeof setTimeout> | null = null;

    // Re-seed the form whenever a fresh project loads.
    $effect(() => {
        if (p) form.hydrate(p as EditableProject);
    });

    async function persist() {
        if (!projectId) return;
        if (saveTimer) {
            clearTimeout(saveTimer);
            saveTimer = null;
        }
        const ok = await form.save(senseiApi(appState.port), projectId);
        if (ok) {
            saveTimer = setTimeout(() => {
                if (form.saveStatus === 'saved') form.saveStatus = 'idle';
            }, 1500);
        }
    }
</script>

{#snippet bindingChip(bg: string, text: string, label: string, state: string)}
    <span
        class="font-mono text-xs px-2 py-0.5 rounded-full {bg} {text}"
        data-binding-state={state}
    >{label}</span>
{/snippet}

{#snippet confirmedRow(b: ConfirmedBinding)}
    <div class="flex items-center gap-3 border border-paper-edge rounded-lg bg-paper-soft p-4">
        <Kanji char={b.kanji} size="xl" tone="accent" />
        <div class="flex-1 min-w-0">
            <div class="font-mono text-xs text-ink-mute truncate">{b.tenantKey}</div>
            <div class="font-mono text-xs text-ink-mute truncate mt-0.5">{b.dojoUrl}</div>
        </div>
        {@render bindingChip(confirmedChip().bg, confirmedChip().text, confirmedChip().label, 'confirmed')}
    </div>
{/snippet}

{#snippet inferredRow(b: InferredBinding)}
    <div class="flex items-center gap-3 border border-paper-edge rounded-lg bg-paper-soft p-4">
        <Kanji char={b.kanji} size="xl" tone="muted" />
        <div class="flex-1 min-w-0">
            <div class="text-sm text-ink truncate">matched org · {b.matchedSlug}</div>
            <div class="font-mono text-xs text-ink-mute truncate mt-0.5">{b.tenantKey}</div>
        </div>
        {@render bindingChip(inferredChip().bg, inferredChip().text, inferredChip().label, 'inferred')}
        <Button
            variant="primary"
            size="sm"
            data-testid="binding-confirm"
            disabled={bindAction.pending}
            onclick={() => confirmBinding(b)}
        >{bindAction.pending ? 'Confirming…' : 'Confirm'}</Button>
    </div>
{/snippet}

<PageHeader title={p?.name ?? "—"} description={p?.goal ?? p?.vision ?? undefined} />
<div class="px-6 py-6 max-w-[600px]">
    <section
        class="px-7 py-7 bg-paper-mute border border-paper-mute rounded-lg"
        data-testid="project-metadata"
    >
        <div class="flex items-baseline justify-between mb-4">
            <h3 class="text-base m-0">Identity</h3>
            <span
                class="text-xs"
                class:text-ink-mute={form.saveStatus === 'idle'}
                class:text-ink-soft={form.saveStatus === 'saving'}
                class:text-success={form.saveStatus === 'saved'}
                class:text-warning={form.saveStatus === 'error'}
                data-testid="metadata-save-status"
            >{saveStatusLabel(form.saveStatus, form.saveError)}</span>
        </div>

        <div class="flex flex-col divide-y divide-paper-edge">
            <div class="grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                <div>
                    <div class="text-sm text-ink">Status</div>
                    <div class="text-xs text-ink-mute mt-0.5">
                        Lifecycle stage — from discovery through to archived.
                    </div>
                </div>
                <div data-testid="metadata-maturity" class="min-w-[140px]">
                    <Select
                        class="text-xs"
                        items={maturityItems}
                        value={form.maturity}
                        disabled={!p}
                        onchange={(v) => {
                            form.maturity = v as string;
                            void persist();
                        }}
                    />
                </div>
            </div>

            <div class="grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                <div>
                    <div class="text-sm text-ink">Client</div>
                    <div class="text-xs text-ink-mute mt-0.5">
                        Who this work is for. Optional.
                    </div>
                </div>
                <input
                    type="text"
                    class="w-[220px] px-3 py-2 text-sm border border-paper-mute rounded-md bg-paper-soft text-ink outline-none text-right"
                    data-testid="metadata-client"
                    disabled={!p}
                    value={form.client}
                    oninput={(e) => {
                        form.client = e.currentTarget.value;
                        void persist();
                    }}
                    placeholder="client / organisation"
                />
            </div>

            <div class="grid grid-cols-[1fr_auto] gap-6 items-start py-3">
                <div>
                    <div class="text-sm text-ink">Goal</div>
                    <div class="text-xs text-ink-mute mt-0.5">
                        What this project is for — its purpose in a line or two.
                    </div>
                </div>
                <textarea
                    rows="2"
                    class="w-[260px] px-3 py-2 text-sm border border-paper-mute rounded-md bg-paper-soft text-ink outline-none resize-y"
                    data-testid="metadata-goal"
                    disabled={!p}
                    value={form.goal}
                    oninput={(e) => {
                        form.goal = e.currentTarget.value;
                        void persist();
                    }}
                    placeholder="short goal or purpose"
                ></textarea>
            </div>

            <div class="grid grid-cols-[1fr_auto] gap-6 items-start py-3">
                <div>
                    <div class="text-sm text-ink">Description</div>
                    <div class="text-xs text-ink-mute mt-0.5">
                        A fuller description of the project. Optional.
                    </div>
                </div>
                <textarea
                    rows="3"
                    class="w-[260px] px-3 py-2 text-sm border border-paper-mute rounded-md bg-paper-soft text-ink outline-none resize-y"
                    data-testid="metadata-description"
                    disabled={!p}
                    value={form.description}
                    oninput={(e) => {
                        form.description = e.currentTarget.value;
                        void persist();
                    }}
                    placeholder="describe this project"
                ></textarea>
            </div>
        </div>
    </section>

    <section class="mt-5" data-testid="project-bindings">
        <h3 class="text-xs font-semibold text-ink-mute m-0 mb-2 uppercase tracking-wide">
            Bindings
        </h3>
        <p class="text-xs text-ink-mute leading-normal mb-3">
            A default is inferred from each git remote and confirmed at first
            scan — never silently.
        </p>
        {#if bindingView.state === 'confirmed'}
            {@render confirmedRow(bindingView)}
        {:else if bindingView.state === 'inferred'}
            {@render inferredRow(bindingView)}
            {#if bindAction.error}
                <p class="text-xs text-danger m-0 mt-2" data-binding-error>{bindAction.error}</p>
            {/if}
        {:else}
            <p class="text-sm text-ink-faint m-0">
                no dōjō binding — this project's findings stay local.
            </p>
        {/if}
    </section>

    <section class="mt-5">
        <h3 class="text-xs font-semibold text-ink-mute m-0 mb-2 uppercase tracking-wide">
            Repos ({data.repos.length})
        </h3>
        <ul class="list-none m-0 p-0">
            {#each data.repos as repo (repo.id)}
                <li class="repo-row flex gap-3 py-1.5 text-sm border-b border-paper-mute">
                    <span class="font-semibold">{repo.name}</span>
                    <span class="opacity-50 text-xs font-mono overflow-hidden text-ellipsis">{repo.path}</span>
                </li>
            {/each}
        </ul>
    </section>

    {#if p?.stack}
        <section class="mt-5">
            <h3 class="text-xs font-semibold text-ink-mute m-0 mb-2 uppercase tracking-wide">
                Stack
            </h3>
            <div class="flex flex-wrap gap-1.5">
                {#each [...(p.stack?.languages ?? []), ...(p.stack?.frameworks ?? [])] as t (t)}
                    <span class="bg-paper-mute text-xs px-2 py-1 rounded-md">{t}</span>
                {/each}
            </div>
        </section>
    {/if}
</div>

<style>
    .repo-row:last-child {
        border-bottom: none;
    }
</style>
