<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageData, ActionData } from './$types';

  const { data, form }: { data: PageData; form: ActionData } = $props();

  const fmt = (iso: string | null) =>
    iso ? new Date(iso).toLocaleString() : '—';

  const isExpired = (expiresAt: string | null) =>
    expiresAt !== null && new Date(expiresAt) < new Date();
</script>

<a href="/repos">← Repos</a>
<h1>Invitations</h1>
<p style="color:#64748b;font-size:0.875rem">Pending invitations for this account</p>

<!-- Invite form -->
<div class="invite-card">
  <h2>Invite a team member</h2>
  <form method="POST" action="?/invite" use:enhance>
    <div class="form-row">
      <div class="field">
        <label for="email">Email address</label>
        <input
          id="email"
          name="email"
          type="email"
          placeholder="user@example.com"
          required
        />
      </div>
      <div class="field field-role">
        <label for="role">Role</label>
        <select id="role" name="role">
          <option value="member">Member</option>
          <option value="admin">Admin</option>
        </select>
      </div>
      <button type="submit" class="btn-primary">Send invite</button>
    </div>
    {#if form?.error}
      <p class="form-error">{form.error}</p>
    {/if}
    {#if form?.success}
      <p class="form-success">Invitation sent to {form.email}.</p>
    {/if}
  </form>
</div>

<!-- Pending invitations table -->
<h2 style="margin-top:32px">Pending ({data.invitations.length})</h2>

{#if data.invitations.length === 0}
  <p style="color:#64748b">No pending invitations. Use the form above to invite team members.</p>
{:else}
  <table class="inv-table">
    <thead>
      <tr>
        <th>Email</th>
        <th>Role</th>
        <th>Invited</th>
        <th>Expires</th>
        <th>Status</th>
      </tr>
    </thead>
    <tbody>
      {#each data.invitations as inv (inv.id)}
        {@const expired = isExpired(inv.expiresAt)}
        <tr>
          <td class="mono" style="font-size:0.875rem">{inv.email}</td>
          <td>
            <span
              class="badge"
              style={inv.role === 'admin' || inv.role === 'owner'
                ? 'background:#ede9fe;color:#5b21b6'
                : 'background:#f1f5f9;color:#475569'}
            >{inv.role}</span>
          </td>
          <td style="color:#64748b;font-size:0.8rem;white-space:nowrap">{fmt(inv.createdAt)}</td>
          <td style="font-size:0.8rem;white-space:nowrap;color:{expired ? '#991b1b' : '#64748b'}">
            {fmt(inv.expiresAt)}
          </td>
          <td>
            {#if expired}
              <span class="badge" style="background:#fee2e2;color:#991b1b">Expired</span>
            {:else}
              <span class="badge" style="background:#dbeafe;color:#1d4ed8">Pending</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  h1 { font-size: 1.5rem; font-weight: 700; color: #1e293b; margin: 8px 0 4px; }
  h2 { font-size: 1rem; font-weight: 600; color: #374151; margin: 0 0 12px; }

  .invite-card {
    margin-top: 24px;
    padding: 20px 24px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
  }
  .form-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 200px;
  }
  .field-role { flex: 0 0 120px; min-width: 120px; }
  label { font-size: 0.75rem; font-weight: 600; color: #475569; text-transform: uppercase; letter-spacing: 0.03em; }
  input, select {
    padding: 7px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #1e293b;
    background: #fff;
    outline: none;
  }
  input:focus, select:focus { border-color: #3b82f6; box-shadow: 0 0 0 2px #bfdbfe; }

  .btn-primary {
    padding: 8px 20px;
    background: #2563eb;
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    align-self: flex-end;
  }
  .btn-primary:hover { background: #1d4ed8; }

  .form-error { margin-top: 8px; color: #991b1b; font-size: 0.875rem; }
  .form-success { margin-top: 8px; color: #166534; font-size: 0.875rem; }

  .inv-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  .inv-table th {
    text-align: left;
    padding: 6px 12px 6px 0;
    border-bottom: 2px solid #e2e8f0;
    color: #64748b;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .inv-table td { padding: 10px 12px 10px 0; border-bottom: 1px solid #f1f5f9; }
  .inv-table tr:hover td { background: #f8fafc; }
  .mono { font-family: monospace; }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
  }
</style>
