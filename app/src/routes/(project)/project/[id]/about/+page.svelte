<script lang="ts">
    import { PageHeader, Kanji } from '$lib/components';
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

    // Editable copy of the identity-level fields the daemon persists in the
    // project settings JSONB. Read-only fields (id, timestamps, stack derived
    // from indexing) stay off this form.
    let editing = $state(false);
    let saving = $state(false);
    let saveError = $state<string | null>(null);
    let form = $state({
        name: '',
        client: '',
        goal: '',
        preferred_acp: '',
        maturity: '',
    });

    // Reset the form whenever a fresh project loads.
    $effect(() => {
        if (p) {
            form = {
                name: p.name ?? '',
                client: p.client ?? '',
                goal: p.goal ?? '',
                preferred_acp: p.preferred_acp ?? '',
                maturity: p.maturity ?? '',
            };
        }
    });

    async function save() {
        if (!projectId) return;
        saving = true;
        saveError = null;
        try {
            // Skip empty strings so the JSONB patch merged by the daemon stays
            // lossless — unset fields don't clobber existing values.
            const patch: Record<string, string> = {};
            for (const [k, v] of Object.entries(form)) {
                if (v && v.length > 0) patch[k] = v;
            }
            await senseiApi(appState.port).updateProject(projectId, patch);
            editing = false;
        } catch (err: unknown) {
            saveError = err instanceof Error ? err.message : String(err);
        } finally {
            saving = false;
        }
    }

    function cancelEdit() {
        if (p) {
            form = {
                name: p.name ?? '',
                client: p.client ?? '',
                goal: p.goal ?? '',
                preferred_acp: p.preferred_acp ?? '',
                maturity: p.maturity ?? '',
            };
        }
        editing = false;
        saveError = null;
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
        <button
            type="button"
            data-testid="binding-confirm"
            disabled={bindAction.pending}
            onclick={() => confirmBinding(b)}
            class="px-3 py-1.5 rounded-md border-none bg-primary text-on-primary text-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        >{bindAction.pending ? 'Confirming…' : 'Confirm'}</button>
    </div>
{/snippet}

<PageHeader title={p?.name ?? "—"} description={p?.goal ?? undefined} />
<div class="px-6 py-6 max-w-[600px]">
    {#if !editing}
        {#if p?.client}<p class="text-sm text-ink-mute mb-3">Client: {p.client}</p>{/if}

        <section class="mt-5" data-testid="project-bindings">
            <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
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
            <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
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
                <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
                    Stack
                </h3>
                <div class="flex flex-wrap gap-1.5">
                    {#each [...(p.stack?.languages ?? []), ...(p.stack?.frameworks ?? [])] as t (t)}
                        <span class="bg-paper-mute text-xs px-2 py-1 rounded-md">{t}</span>
                    {/each}
                </div>
            </section>
        {/if}

        <div class="mt-6">
            <button
                type="button"
                class="px-3 py-1.5 rounded-md text-xs border border-paper-edge bg-transparent text-ink-soft cursor-pointer"
                onclick={() => (editing = true)}
                disabled={!p}
            >Edit identity</button>
        </div>
    {:else}
        <form
            onsubmit={(e) => {
                e.preventDefault();
                save();
            }}
            class="flex flex-col gap-4"
        >
            {#each [
                { key: 'name', label: 'Name', placeholder: 'Project name' },
                { key: 'client', label: 'Client', placeholder: 'Client / organisation' },
                { key: 'goal', label: 'Goal', placeholder: 'Short goal or purpose' },
                { key: 'preferred_acp', label: 'Preferred assistant', placeholder: 'claude-code | zed | ...' },
                { key: 'maturity', label: 'Maturity', placeholder: 'spike | mvp | mature | archived' },
            ] as field (field.key)}
                <label class="flex flex-col gap-1">
                    <span class="text-xs uppercase tracking-wide opacity-60">{field.label}</span>
                    <input
                        type="text"
                        placeholder={field.placeholder}
                        class="px-3 py-2 border border-paper-edge rounded-md bg-paper text-ink text-sm outline-none"
                        bind:value={form[field.key as keyof typeof form]}
                    />
                </label>
            {/each}

            {#if saveError}
                <p class="text-xs text-danger m-0">Save failed: {saveError}</p>
            {/if}

            <div class="flex gap-2">
                <button
                    type="submit"
                    disabled={saving}
                    class="px-4 py-2 rounded-md text-sm bg-primary text-on-primary border-none cursor-pointer"
                >{saving ? 'Saving…' : 'Save'}</button>
                <button
                    type="button"
                    onclick={cancelEdit}
                    disabled={saving}
                    class="px-4 py-2 rounded-md text-sm bg-transparent text-ink-soft border border-paper-edge cursor-pointer"
                >Cancel</button>
            </div>
        </form>
    {/if}
</div>

<style>
    .repo-row:last-child {
        border-bottom: none;
    }
</style>
