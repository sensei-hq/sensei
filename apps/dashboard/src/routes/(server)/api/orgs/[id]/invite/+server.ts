import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';

export const POST: RequestHandler = async ({ request, fetch, locals, params }) => {
  const userId = locals.session?.user?.id;
  if (!userId) throw error(401, 'Not authenticated');

  // Verify caller is admin/owner of this account
  const membershipRes = await fetch(
    `/data/core/profile_accounts?user_id=eq.${userId}&account_id=eq.${params.id}&select=role&limit=1`,
  );
  const { data: memberships } = await membershipRes.json();
  const role = memberships?.[0]?.role;
  if (!role || !['admin', 'owner'].includes(role)) throw error(403, 'Only admins can invite members');

  const body = await request.json() as { email: string; role?: string };
  if (!body.email?.trim()) throw error(400, 'Email is required');
  const inviteRole = body.role === 'admin' ? 'admin' : 'member';

  // Insert invitation record
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString();
  const inviteRes = await fetch('/data/core/invitations', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ account_id: params.id, email: body.email.trim(), role: inviteRole, expires_at: expiresAt }),
  });
  if (!inviteRes.ok) throw error(500, 'Failed to create invitation');

  // Send invite email via Supabase admin
  const supabaseUrl = env.SUPABASE_URL;
  const serviceKey  = env.SUPABASE_SERVICE_ROLE_KEY;
  if (supabaseUrl && serviceKey) {
    await fetch(`${supabaseUrl}/auth/v1/admin/users`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${serviceKey}`,
        apikey: serviceKey,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ email: body.email.trim(), invite: true }),
    });
  }

  return json({ ok: true });
};
