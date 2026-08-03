-- Seed test users for LOCAL Dōjō development only (localhost supabase stack).
-- Loaded by `supabase db reset` / first `supabase start` (see [db.seed] in config.toml).
-- These are throwaway local credentials — no real secrets, never used in production.
-- Sign in via magic link (Inbucket catches the email at http://127.0.0.1:54324) or
-- with the password below. `app_metadata.role` is the claim the dōjō service reads
-- (verify_supabase_jwt → dojo_role_to_access) to gate the admin / lead consoles.
--
-- Password for every seeded user: sensei-local
--
-- Roles cover the console personas: admin, maintainer, contributor, lead.

DO $$
DECLARE
  u RECORD;
BEGIN
  FOR u IN
    SELECT * FROM (VALUES
      ('00000000-0000-0000-0000-000000000001'::uuid, 'admin@sensei.test',       'admin'),
      ('00000000-0000-0000-0000-000000000002'::uuid, 'maintainer@sensei.test',  'maintainer'),
      ('00000000-0000-0000-0000-000000000003'::uuid, 'lead@sensei.test',        'lead'),
      ('00000000-0000-0000-0000-000000000004'::uuid, 'contributor@sensei.test', 'contributor')
    ) AS t(id, email, role)
  LOOP
    INSERT INTO auth.users (
      id, instance_id, aud, role, email, encrypted_password,
      email_confirmed_at, created_at, updated_at,
      raw_app_meta_data, raw_user_meta_data, is_super_admin,
      -- GoTrue scans these token columns into Go `string` (not sql.NullString),
      -- so a NULL — the column default when omitted — makes every sign-in 500
      -- with "Database error querying schema". Seed them empty, as GoTrue does.
      confirmation_token, recovery_token, email_change,
      email_change_token_new, email_change_token_current,
      phone_change, phone_change_token, reauthentication_token
    ) VALUES (
      u.id,
      '00000000-0000-0000-0000-000000000000',
      'authenticated', 'authenticated',
      u.email,
      crypt('sensei-local', gen_salt('bf')),
      now(), now(), now(),
      jsonb_build_object('provider', 'email', 'providers', ARRAY['email'], 'role', u.role),
      jsonb_build_object('email_verified', true),
      false,
      '', '', '',
      '', '',
      '', '', ''
    ) ON CONFLICT (id) DO NOTHING;

    INSERT INTO auth.identities (
      id, user_id, identity_data, provider, provider_id, created_at, updated_at, last_sign_in_at
    ) VALUES (
      u.id,
      u.id,
      jsonb_build_object('sub', u.id::text, 'email', u.email),
      'email',
      u.id::text,
      now(), now(), now()
    ) ON CONFLICT (id) DO NOTHING;
  END LOOP;
END $$;
