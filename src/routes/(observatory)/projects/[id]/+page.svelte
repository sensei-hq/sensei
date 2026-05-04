<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  let projectId = $derived($page.params.id);

  type Project = {
    id: string;
    name: string;
    client?: string;
    goal?: string;
    maturity: string;
    stack: { languages?: string[]; frameworks?: string[]; runtimes?: string[]; services?: string[] };
    icon?: { value?: string };
    preferred_acp?: string;
  };

  type Repo = {
    repo_id: string;
    name: string;
    path: string;
    role?: string;
    language?: string;
  };

  let project = $state<Project | null>(null);
  let repos = $state<Repo[]>([]);
  let loading = $state(true);
  let tab = $state<'overview' | 'graph' | 'patterns' | 'sessions' | 'settings'>('overview');

  onMount(async () => {
    await appState.load();
    const api = senseiApi(appState.port);
    const projects = await api.listProjects();
    project = projects.find((p: any) => p.id === projectId) ?? null;
    const allRepos = await api.getRepos();
    repos = allRepos.filter((r: any) => r.project_id === projectId);
    loading = false;
  });

  let kanji = $derived(project?.icon?.value ?? '場');
  let stackTags = $derived([
    ...(project?.stack?.languages ?? []),
    ...(project?.stack?.frameworks ?? []),
  ]);
</script>

<div class="page">
  {#if loading}
    <p class="hint">Loading project...</p>
  {:else if !project}
    <div class="empty-state">
      <span class="kanji empty-kanji">場</span>
      <p class="display empty-title">Project not found.</p>
    </div>
  {:else}
    <!-- Project header -->
    <header class="project-header">
      <div class="header-top">
        <span class="kanji header-kanji">{kanji}</span>
        <div class="header-info">
          <h1 class="display project-name">{project.name}</h1>
          {#if project.client}
            <span class="project-client">{project.client}</span>
          {/if}
        </div>
        <span class="maturity-tag">{project.maturity}</span>
      </div>
      {#if project.goal}
        <p class="project-goal">{project.goal}</p>
      {/if}
      {#if stackTags.length > 0}
        <div class="stack-tags">
          {#each stackTags as tag}
            <span class="stack-tag">{tag}</span>
          {/each}
        </div>
      {/if}
    </header>

    <!-- Tab bar -->
    <div class="tab-bar">
      {#each [['overview', 'Overview'], ['graph', 'Graph'], ['patterns', 'Patterns'], ['sessions', 'Sessions'], ['settings', 'Settings']] as [key, label]}
        <button class="tab" class:active={tab === key} onclick={() => tab = key as any}>{label}</button>
      {/each}
    </div>

    <!-- Tab content -->
    {#if tab === 'overview'}
      <div class="tab-content">
        <!-- Repos -->
        <div class="section">
          <h3 class="section-title">Repositories</h3>
          {#if repos.length === 0}
            <p class="hint">No repositories linked to this project.</p>
          {:else}
            <div class="repo-list">
              {#each repos as repo (repo.repo_id)}
                <div class="repo-row">
                  <span class="repo-name">{repo.name}</span>
                  <span class="repo-path">{repo.path}</span>
                  {#if repo.role}
                    <span class="repo-role">{repo.role}</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Recommendations placeholder -->
        <div class="section">
          <h3 class="section-title">Recommendations</h3>
          <div class="placeholder-card">
            <span class="kanji" style="font-size: 24px; color: var(--shu); opacity: 0.4;">薦</span>
            <p class="hint">Recommendations will appear once sensei has observed enough sessions in this project.</p>
          </div>
        </div>
      </div>

    {:else if tab === 'graph'}
      <div class="tab-content">
        <div class="placeholder-card large">
          <span class="kanji" style="font-size: 48px; color: var(--shu); opacity: 0.3;">紋</span>
          <p class="display" style="font-size: 18px; font-weight: 400; margin: 16px 0 8px;">Code graph</p>
          <p class="hint">Interactive code graph with three lenses: Complexity, Rework, Staleness. Nodes sized by fan-in, colored by overlay.</p>
        </div>
      </div>

    {:else if tab === 'patterns'}
      <div class="tab-content">
        <div class="section">
          <h3 class="section-title">Followed patterns</h3>
          <p class="hint">Detected patterns (Adapter, Observer, Factory, Repository) will appear once the indexing pipeline has analyzed this project's code.</p>
        </div>
        <div class="section">
          <h3 class="section-title">Anti-patterns</h3>
          <p class="hint">Anti-patterns (duplication, god-nodes, dead code) with severity and suggested fixes will appear here after indexing.</p>
        </div>
      </div>

    {:else if tab === 'sessions'}
      <div class="tab-content">
        <div class="placeholder-card">
          <span class="kanji" style="font-size: 32px; color: var(--shu); opacity: 0.4;">刻</span>
          <p class="hint">Sessions scoped to this project. Same format as the observatory sessions view, filtered to show only sessions in {project.name}.</p>
        </div>
      </div>

    {:else if tab === 'settings'}
      <div class="tab-content">
        <div class="section">
          <h3 class="section-title">Identity</h3>
          <div class="settings-grid">
            <span class="setting-label">Name</span>
            <span class="setting-val">{project.name}</span>
            <span class="setting-label">Client</span>
            <span class="setting-val">{project.client ?? '—'}</span>
            <span class="setting-label">Goal</span>
            <span class="setting-val">{project.goal ?? '—'}</span>
            <span class="setting-label">Preferred ACP</span>
            <span class="setting-val">{project.preferred_acp ?? '—'}</span>
            <span class="setting-label">Maturity</span>
            <span class="setting-val">{project.maturity}</span>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page {
    max-width: 820px;
    margin: 0 auto;
    padding: 48px 48px 64px;
  }

  /* ── Header ─────────────────────────────────────────────── */
  .project-header { margin-bottom: 28px; }
  .header-top { display: flex; align-items: center; gap: 16px; margin-bottom: 8px; }
  .header-kanji { font-size: 36px; color: var(--shu); opacity: 0.7; }
  .header-info { flex: 1; }
  .project-name { font-size: 24px; font-weight: 400; margin: 0; }
  .project-client { font-size: 12px; color: var(--sumi-3); }
  .maturity-tag {
    padding: 4px 12px;
    border-radius: 100px;
    font-size: 11px;
    background: var(--paper-3);
    color: var(--sumi-3);
    text-transform: capitalize;
  }
  .project-goal { font-size: 14px; color: var(--sumi-2); margin: 0 0 12px; line-height: 1.5; }
  .stack-tags { display: flex; gap: 6px; flex-wrap: wrap; }
  .stack-tag {
    padding: 3px 10px;
    border-radius: 100px;
    font-size: 11px;
    background: var(--paper-3);
    color: var(--sumi-3);
  }

  /* ── Tabs ────────────────────────────────────────────────── */
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: var(--hairline);
    margin-bottom: 28px;
  }
  .tab {
    padding: 8px 18px;
    border: none;
    background: none;
    color: var(--sumi-3);
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .tab:hover { color: var(--sumi-2); }
  .tab.active { color: var(--sumi); border-bottom-color: var(--shu); }

  /* ── Content ────────────────────────────────────────────── */
  .tab-content { display: flex; flex-direction: column; gap: 28px; }
  .section-title { font-size: 14px; margin: 0 0 14px; color: var(--sumi); }

  /* ── Repos ──────────────────────────────────────────────── */
  .repo-list { display: flex; flex-direction: column; gap: 2px; }
  .repo-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius);
    transition: background 0.1s;
  }
  .repo-row:hover { background: var(--paper-2); }
  .repo-name { font-size: 13px; font-weight: 500; color: var(--sumi); }
  .repo-path { font-size: 12px; color: var(--sumi-3); font-family: var(--font-mono); flex: 1; }
  .repo-role {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--sumi-4);
    padding: 2px 8px;
    border-radius: 100px;
    background: var(--paper-3);
  }

  /* ── Placeholders ───────────────────────────────────────── */
  .placeholder-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 40px 20px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
    text-align: center;
  }
  .placeholder-card.large { padding: 80px 20px; }
  .hint { font-size: 13px; color: var(--sumi-3); line-height: 1.65; max-width: 420px; margin: 0; }

  /* ── Settings grid ──────────────────────────────────────── */
  .settings-grid {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 8px 16px;
  }
  .setting-label { font-size: 12px; color: var(--sumi-3); }
  .setting-val { font-size: 13px; color: var(--sumi); }

  /* ── Empty state ────────────────────────────────────────── */
  .empty-state { text-align: center; padding: 80px 20px; }
  .empty-kanji { font-size: 64px; color: var(--shu); opacity: 0.3; }
  .empty-title { font-size: 20px; font-weight: 400; margin: 16px 0 8px; }
</style>
