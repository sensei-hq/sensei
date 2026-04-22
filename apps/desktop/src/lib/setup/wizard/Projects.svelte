<script lang="ts">
  import type { WizardState, WizUpdate } from '../types.js';
  import { ROLES } from '../types.js';

  let { wizState, update }: {
    wizState: WizardState;
    update: WizUpdate;
  } = $props();

  function toggleConfirm(projectId: string) {
    const next = wizState.projects.map(p =>
      p.id === projectId ? { ...p, confirmed: !p.confirmed } : p
    );
    update({ projects: next });
  }

  function setRole(repoId: string, roleId: string) {
    update({ roles: { ...wizState.roles, [repoId]: roleId } });
  }

  function roleFor(repoId: string) {
    const id = wizState.roles[repoId] ?? 'backend';
    return ROLES.find(r => r.id === id) ?? ROLES[0];
  }

  function roleColor(roleId: string): string {
    switch (roleId) {
      case 'frontend': return 'var(--shu)';
      case 'backend':  return 'var(--jade)';
      case 'library':  return 'var(--amber)';
      case 'docs':     return 'var(--sumi-3)';
      case 'infra':    return 'var(--sumi-2)';
      default:         return 'var(--sumi-3)';
    }
  }

  function roleBg(roleId: string): string {
    switch (roleId) {
      case 'frontend': return 'var(--shu-soft)';
      case 'backend':  return 'var(--jade-soft)';
      case 'library':  return 'var(--amber-soft)';
      case 'docs':     return 'var(--paper-3)';
      case 'infra':    return 'var(--paper-3)';
      default:         return 'var(--paper-3)';
    }
  }
</script>

<section class="step">
  <div class="step-label"><span class="kanji">六</span> STEP</div>
  <h1 class="display headline">Projects</h1>
  <p class="subtitle">A project has one or more repos. Edit, split, or confirm.</p>
  <p class="instruction">
    A single-repo project is the default. Multi-repo projects are auto-grouped from
    sibling folders and name patterns. Split when they shouldn't be together.
  </p>

  <div class="project-cards">
    {#each wizState.projects as project}
      {@const isMulti = project.repos.length > 1}
      <div class="card" class:confirmed={project.confirmed}>
        <!-- Card header -->
        <div class="card-header">
          <span class="card-kanji kanji">{project.kanji}</span>
          <div class="card-info">
            <div class="card-title">{project.name}</div>
            <div class="card-meta">
              {project.path} &middot; {project.repos.length} repo{project.repos.length !== 1 ? 's' : ''}
            </div>
          </div>

          <div class="card-actions">
            {#if isMulti}
              <span class="multi-badge">MULTI-REPO</span>
            {/if}
            <button class="action-link">merge...</button>
            <button class="action-link">edit &middot;</button>
            {#if isMulti}
              <button class="action-link">split all into {project.repos.length} projects</button>
            {/if}
            <button
              class="confirm-check"
              class:checked={project.confirmed}
              onclick={() => toggleConfirm(project.id)}
              title={project.confirmed ? 'Unconfirm' : 'Confirm'}
            >
              {#if project.confirmed}
                <span class="check-icon">&#10003;</span>
              {/if}
            </button>
          </div>
        </div>

        <!-- Repo rows -->
        <div class="repo-strip">
          {#each project.repos as repo}
            {@const role = roleFor(repo.id)}
            {@const rid = wizState.roles[repo.id] ?? repo.suggestedRole}
            <div class="repo-pill">
              <span class="repo-name">{repo.name}</span>
              <span class="repo-files">{repo.files}f</span>
              <!-- svelte-ignore a11y_no_onchange -->
              <select
                class="role-select"
                value={rid}
                onchange={(e) => setRole(repo.id, (e.target as HTMLSelectElement).value)}
                style="color: {roleColor(rid)}; background: {roleBg(rid)};"
              >
                {#each ROLES as r}
                  <option value={r.id}>{r.label}</option>
                {/each}
              </select>
              <span class="role-dash">&mdash;</span>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <p class="footer-note">
    More options &mdash; external integrations, clients, custom rules &mdash; per project later from its Settings.
  </p>
</section>

<style>
  .step {
    padding: var(--space-10) var(--space-12);
    max-width: 860px;
  }

  .step-label {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    margin-bottom: var(--space-2);
  }

  .step-label .kanji {
    color: var(--shu);
    margin-right: 4px;
  }

  .headline {
    font-size: 40px;
    color: var(--sumi);
    margin: 0 0 var(--space-2) 0;
    line-height: 1.15;
  }

  .subtitle {
    font-size: 15px;
    color: var(--sumi-3);
    margin: 0 0 var(--space-3) 0;
  }

  .instruction {
    font-size: 13px;
    color: var(--sumi-3);
    line-height: 1.6;
    margin: 0 0 var(--space-8) 0;
    max-width: 620px;
  }

  /* ── Project cards ────────────────────────────────────────── */

  .project-cards {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .card {
    background: var(--paper-2);
    border-radius: var(--radius-lg);
    border: 1px solid var(--paper-edge);
    overflow: hidden;
    transition: border-color 0.14s;
  }

  .card.confirmed {
    border-color: var(--paper-edge);
  }

  .card-header {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
  }

  .card-kanji {
    font-size: 28px;
    color: var(--sumi-3);
    flex-shrink: 0;
    line-height: 1;
    margin-top: 2px;
  }

  .card-info {
    flex: 1;
    min-width: 0;
  }

  .card-title {
    font-family: var(--font-display);
    font-feature-settings: "ss01";
    font-size: 20px;
    font-weight: 500;
    color: var(--sumi);
    line-height: 1.25;
  }

  .card-meta {
    font-size: 12px;
    color: var(--sumi-3);
    margin-top: 3px;
    font-family: var(--font-mono);
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-shrink: 0;
  }

  .multi-badge {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--shu);
    background: var(--shu-soft);
    padding: 3px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .action-link {
    font-size: 12px;
    color: var(--sumi-3);
    background: none;
    border: none;
    cursor: pointer;
    font-family: var(--font-ui);
    padding: 0;
    white-space: nowrap;
    transition: color 0.14s;
  }

  .action-link:hover {
    color: var(--sumi);
  }

  .confirm-check {
    width: 24px;
    height: 24px;
    border-radius: var(--radius);
    border: 2px solid var(--paper-edge);
    background: var(--paper);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: all 0.14s;
  }

  .confirm-check.checked {
    background: var(--sumi);
    border-color: var(--sumi);
  }

  .check-icon {
    color: var(--paper);
    font-size: 13px;
    font-weight: 600;
    line-height: 1;
  }

  /* ── Repo strip ───────────────────────────────────────────── */

  .repo-strip {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-6) var(--space-5);
  }

  .repo-pill {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .repo-name {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--sumi-2);
    font-weight: 500;
  }

  .repo-files {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--sumi-4);
  }

  .role-select {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-ui);
    border: none;
    border-radius: 4px;
    padding: 2px 6px;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    outline: none;
    letter-spacing: 0.02em;
  }

  .role-dash {
    color: var(--sumi-4);
    font-size: 12px;
  }

  /* ── Footer ───────────────────────────────────────────────── */

  .footer-note {
    margin-top: var(--space-8);
    font-size: 13px;
    color: var(--sumi-3);
    line-height: 1.6;
  }
</style>
