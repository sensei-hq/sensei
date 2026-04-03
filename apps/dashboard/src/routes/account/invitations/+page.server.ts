import type { PageServerLoad, Actions } from './$types';
import { createClient } from '@supabase/supabase-js';
import { SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY } from '$env/static/private';
import { fail } from '@sveltejs/kit';

function getCoreDb() {
  return createClient(SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, {
    db: { schema: 'core' },
    auth: { persistSession: false },
  });
}

export interface Invitation {
  id: string;
  email: string;
  role: string;
  expiresAt: string | null;
  createdAt: string;
}

export const load: PageServerLoad = async () => {
  let invitations: Invitation[] = [];

  try {
    const db = getCoreDb();

    const { data } = await db
      .from('invitations')
      .select('id, email, role, expires_at, created_at')
      .is('accepted_at', null)
      .order('created_at', { ascending: false });

    invitations = ((data ?? []) as Array<{
      id: string;
      email: string;
      role: string | null;
      expires_at: string | null;
      created_at: string;
    }>).map(inv => ({
      id: inv.id,
      email: inv.email,
      role: inv.role ?? 'member',
      expiresAt: inv.expires_at,
      createdAt: inv.created_at,
    }));
  } catch {
    // Table may not exist yet — return empty data gracefully
  }

  return { invitations };
};

export const actions: Actions = {
  invite: async ({ request }: { request: Request }) => {
    const formData = await request.formData();
    const email = (formData.get('email') as string | null)?.trim();
    const role = (formData.get('role') as string | null)?.trim() ?? 'member';

    if (!email || !email.includes('@')) {
      return fail(400, { error: 'A valid email address is required.' });
    }

    const validRoles = ['member', 'admin'];
    if (!validRoles.includes(role)) {
      return fail(400, { error: 'Role must be one of: member, admin.' });
    }

    try {
      const db = getCoreDb();

      // Set expiry to 7 days from now
      const expiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString();

      const { error: insertError } = await db
        .from('invitations')
        .insert({ email, role, expires_at: expiresAt });

      if (insertError) {
        return fail(500, { error: insertError.message });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create invitation.';
      return fail(500, { error: message });
    }

    return { success: true, email };
  },
};
