import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';

export const GET: RequestHandler = async ({ request, fetch }) => {
  const auth = request.headers.get('authorization') ?? '';
  const token = auth.startsWith('Bearer ') ? auth.slice(7) : null;
  if (!token) throw error(401, 'Missing Bearer token');

  // Validate token with Supabase
  const supabaseUrl = env.SUPABASE_URL ?? env.PUBLIC_SUPABASE_URL;
  const anonKey = env.PUBLIC_SUPABASE_ANON_KEY;

  const headers: Record<string, string> = { Authorization: `Bearer ${token}` };
  if (anonKey) headers['apikey'] = anonKey;

  const userRes = await fetch(`${supabaseUrl}/auth/v1/user`, { headers });

  if (!userRes.ok) throw error(401, 'Invalid or expired token');
  const user = await userRes.json() as { id: string; email: string; app_metadata?: { role?: string } };

  // Fetch primary account membership
  const accountRes = await fetch(
    `/data/core/profile_accounts?user_id=eq.${user.id}&select=role,account:accounts(id,slug,account_type)&limit=1`,
  );
  const { data: memberships } = await accountRes.json();
  const membership = memberships?.[0];

  return json({
    userId:      user.id,
    email:       user.email,
    accountId:   membership?.account?.id   ?? null,
    accountSlug: membership?.account?.slug ?? null,
    accountType: membership?.account?.account_type ?? 'individual',
    role:        membership?.role ?? (user.app_metadata?.role ?? 'member'),
  });
};
