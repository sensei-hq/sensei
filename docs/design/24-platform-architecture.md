---
id: platform-architecture
type: design
created: 2026-04-02
status: proposed
---

# Platform Architecture — Multi-Account SaaS

> Design for evolving sensei from a single-developer local tool into a multi-account SaaS platform — like qlty or code climate, but for AI-assisted development. Captures FTR scores, session costs, tool usage, and code quality signals across teams, then surfaces them back as coaching to improve future AI sessions.

---

## 1. Goal and Mental Model

A developer installs sensei locally and runs their normal workflow. Their sessions produce signals — FTR scores, token costs, tool errors, patterns used. Those signals are synced to a cloud platform organized by **account**.

An account is either an **individual** (one developer, open-source or solo) or a **team** (an org with multiple developers). A team account admin (Alice) sees aggregate quality trends and can invite members. A platform admin sees anonymized cross-account signals to improve the default coaching model. No developer ever sees another account's raw data.

```
  Developer laptop                Cloud Platform
  ────────────────                ──────────────────────────────
  sensei MCP server               ┌─ POST /sync (authenticated)
  ↓ session data                  │
  local collector ─── sync ──────►│  Ingest API
                                  │  ↓ PII-scrub + account partition
                                  │  DB: account-partitioned tables
                                  │  ↓
                                  │  Analytics engine
                                  │  ↓
                                  └─ Dashboard (per-account views)
                                      + Platform admin view
```

---

## 2. Account Schema

### 2.1 Core Tables

```sql
-- schema: core

CREATE TABLE core.accounts (
  id            uuid        PRIMARY KEY DEFAULT gen_random_uuid()
, name          varchar     NOT NULL
, slug          varchar     NOT NULL UNIQUE
, account_type  varchar     NOT NULL DEFAULT 'individual'
    CHECK (account_type IN ('individual', 'team', 'platform'))
, domain        varchar     UNIQUE        -- team accounts only; email domain auto-assignment
, oss_shard     smallint                  -- individual accounts only; 0–15, set on create
, is_platform   boolean     NOT NULL DEFAULT false
, status        varchar     NOT NULL DEFAULT 'trial'
    CHECK (status IN ('active', 'suspended', 'trial'))
, created_at    timestamptz NOT NULL DEFAULT now()
, modified_at   timestamptz NOT NULL DEFAULT now()
, modified_by   varchar     NOT NULL DEFAULT 'system'
);

-- Exactly one platform account allowed
CREATE UNIQUE INDEX accounts_platform_ukey
  ON core.accounts(is_platform)
  WHERE is_platform = true;

-- Maps Supabase auth.users → account
CREATE TABLE core.profile_accounts (
  profile_id   uuid        PRIMARY KEY             -- = auth.users.id
, account_id   uuid        NOT NULL REFERENCES core.accounts(id) ON DELETE RESTRICT
, role         varchar     NOT NULL DEFAULT 'user'
    CHECK (role IN ('platform_admin', 'account_admin', 'user'))
, assigned_at  timestamptz NOT NULL DEFAULT now()
, assigned_by  varchar     NOT NULL DEFAULT 'system'
);

CREATE INDEX idx_profile_accounts_account ON core.profile_accounts(account_id);

-- Per-account encryption key (KEK → DEK → sensitive data)
CREATE TABLE core.account_keys (
  account_id   uuid        PRIMARY KEY REFERENCES core.accounts(id) ON DELETE CASCADE
, encrypted_dek bytea      NOT NULL   -- [12-byte IV][16-byte auth tag][32-byte ciphertext]
, dek_version  integer     NOT NULL DEFAULT 1
, created_at   timestamptz NOT NULL DEFAULT now()
, modified_at  timestamptz NOT NULL DEFAULT now()
, modified_by  varchar     NOT NULL DEFAULT 'system'
);
```

### 2.2 Account Invitations (team accounts only)

```sql
CREATE TABLE core.invitations (
  id          uuid        PRIMARY KEY DEFAULT gen_random_uuid()
, account_id  uuid        NOT NULL REFERENCES core.accounts(id)
, email       varchar     NOT NULL
, role        varchar     NOT NULL DEFAULT 'user'
, token       varchar     NOT NULL UNIQUE            -- emailed link token
, expires_at  timestamptz NOT NULL DEFAULT now() + INTERVAL '7 days'
, accepted_at timestamptz
, invited_by  varchar     NOT NULL
, created_at  timestamptz NOT NULL DEFAULT now()
);
```

### 2.3 Migrate Existing Tables to Account-Partitioned

All 20 existing sensei tables get `account_id` added with composite PKs:

```sql
-- Example: repos
ALTER TABLE repos ADD COLUMN account_id uuid REFERENCES core.accounts(id);
UPDATE repos SET account_id = (SELECT id FROM core.accounts WHERE is_platform = false LIMIT 1);
ALTER TABLE repos ALTER COLUMN account_id SET NOT NULL;
ALTER TABLE repos DROP CONSTRAINT repos_pkey;
ALTER TABLE repos ADD PRIMARY KEY (account_id, id);
-- Then recreate as partitioned table (data migration to new partitioned table required)
```

**Tables to migrate:**

| Table | Notes |
|-------|-------|
| `repos` | Primary account object |
| `sessions` | Session per developer per account |
| `snapshots` | Tied to session → account |
| `events` | Tied to session → account |
| `api_requests` | Token costs per account |
| `symbols` | Per-repo → per-account |
| `chunks` | Per-repo → per-account |
| `docs` | Per-repo → per-account |
| `libraries` | Per-account custom library configs |
| `lib_doc_sections` | Per-library → per-account |
| `memory_items` | Per-repo → per-account |
| `references` | Per-repo → per-account |
| `benchmark_runs` | Per-account |
| `benchmark_reports` | Per-account |
| `task_sessions` | Per-account |
| `task_turns` | Per-task-session → per-account |
| `shared_libs` | Platform-level (no account_id) |
| `shared_lib_sections` | Platform-level (no account_id) |
| `repo_libs` | Per-repo → per-account |
| `repo_libraries` | Per-repo → per-account |

---

## 3. Tiered Partitioning Strategy

PostgreSQL has no hard partition limit, but planning overhead grows with partition count — noticeably above ~1,000–2,000 partitions. With open-source individual users potentially numbering in the tens of thousands, one partition per account is not viable.

The solution is a **two-tier partition model**:

| Account type | Partition strategy | Isolation level |
|---|---|---|
| `team` | Dedicated list partition per account | Physical — own partition |
| `individual` | Shared DEFAULT partition, hash sub-partitioned (16 shards) | Logical — `account_id` column filter |
| `platform` | Dedicated list partition | Physical |

```
Table: sessions  (PARTITION BY LIST (account_id))
  ├── sessions_account_<acme_uuid>       FOR VALUES IN (<acme_uuid>)   ← team
  ├── sessions_account_<devstudio_uuid>  FOR VALUES IN (<devstudio_uuid>) ← team
  ├── sessions_account_<platform_uuid>   FOR VALUES IN (<platform_uuid>)  ← platform
  └── sessions_individual               DEFAULT                           ← all individual accounts
        PARTITION BY HASH (account_id) MODULUS 16
          ├── sessions_individual_0
          ├── sessions_individual_1
          └── ... through sessions_individual_15
```

- Individual accounts fall into `sessions_individual` automatically via DEFAULT — no trigger needed per user.
- The DEFAULT partition is hash-sub-partitioned into 16 shards so no single partition becomes a hotspot.
- Team accounts get dedicated partitions when created — this is a **premium feature**, not a correctness requirement. Isolation for individual accounts is still enforced by `account_id` column filtering in the repository layer.
- When an individual account upgrades to a team plan, their data can be migrated to a dedicated partition as part of the upgrade flow.

### 3.1 Partition Creation Trigger

Fires on `INSERT INTO core.accounts`. Only creates dedicated partitions for `team` and `platform` accounts. Individual accounts fall into the DEFAULT automatically.

```sql
CREATE OR REPLACE FUNCTION core.add_account_partitions()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
  r         record;
  safe_id   text;
  part_name text;
BEGIN
  -- Only team and platform accounts get dedicated partitions
  IF NEW.account_type NOT IN ('team', 'platform') THEN
    RETURN NEW;
  END IF;

  safe_id := replace(NEW.id::text, '-', '_');

  -- Discover all list-partitioned tables in public with an account_id column
  FOR r IN
    SELECT c.table_schema, c.table_name
    FROM information_schema.columns c
    JOIN pg_class pc ON pc.relname = c.table_name
    JOIN pg_namespace pn ON pn.oid = pc.relnamespace AND pn.nspname = c.table_schema
    JOIN pg_partitioned_table pt ON pt.partrelid = pc.oid
    WHERE c.column_name = 'account_id' AND c.table_schema = 'public'
    ORDER BY c.table_name
  LOOP
    part_name := r.table_name || '_account_' || safe_id;
    IF NOT EXISTS (
      SELECT 1 FROM pg_class pc2
      JOIN pg_namespace pn2 ON pn2.oid = pc2.relnamespace AND pn2.nspname = r.table_schema
      WHERE pc2.relname = part_name
    ) THEN
      EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I.%I PARTITION OF %I.%I FOR VALUES IN (%L)',
        r.table_schema, part_name, r.table_schema, r.table_name, NEW.id
      );
    END IF;
  END LOOP;

  RETURN NEW;
END;
$$;

CREATE TRIGGER add_account_partitions_trigger
  AFTER INSERT ON core.accounts
  FOR EACH ROW EXECUTE FUNCTION core.add_account_partitions();
```

### 3.2 OSS Shard Assignment Trigger

Individual accounts get a shard assigned on insert (0–15):

```sql
CREATE OR REPLACE FUNCTION core.assign_oss_shard()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.account_type = 'individual' THEN
    NEW.oss_shard := abs(hashtext(NEW.id::text)) % 16;
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER assign_oss_shard_trigger
  BEFORE INSERT ON core.accounts
  FOR EACH ROW EXECUTE FUNCTION core.assign_oss_shard();
```

The `oss_shard` value is informational — it tells you which hash sub-partition the account's data lives in, useful for targeted maintenance queries.

### 3.3 DEFAULT Partition Bootstrap

Run once during DB setup, before any individual accounts are created:

```sql
-- Create the DEFAULT partitions and their hash sub-partitions for each partitioned table
DO $$
DECLARE
  tbl text;
  i   int;
BEGIN
  FOR tbl IN
    SELECT table_name FROM information_schema.columns
    WHERE column_name = 'account_id' AND table_schema = 'public'
  LOOP
    -- Create DEFAULT partition
    EXECUTE format(
      'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.%I DEFAULT PARTITION BY HASH (account_id)',
      tbl || '_individual', tbl
    );
    -- Create 16 hash sub-partitions
    FOR i IN 0..15 LOOP
      EXECUTE format(
        'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.%I FOR VALUES WITH (modulus 16, remainder %s)',
        tbl || '_individual_' || i, tbl || '_individual', i
      );
    END LOOP;
  END LOOP;
END;
$$;
```

---

## 4. Domain-Based Auto-Assignment (Team Accounts)

When a new user signs up via Supabase Auth, auto-assign them to the team account whose `domain` matches their email domain:

```sql
CREATE OR REPLACE FUNCTION core.assign_account_by_domain()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_account_id uuid;
BEGIN
  IF EXISTS (SELECT 1 FROM core.profile_accounts WHERE profile_id = NEW.id) THEN
    RETURN NEW;
  END IF;

  SELECT id INTO v_account_id
  FROM core.accounts
  WHERE domain = split_part(NEW.email, '@', 2)
    AND account_type = 'team'
    AND status = 'active'
  LIMIT 1;

  IF v_account_id IS NOT NULL THEN
    INSERT INTO core.profile_accounts (profile_id, account_id, assigned_by)
    VALUES (NEW.id, v_account_id, 'domain_trigger');
  ELSE
    -- No matching team domain → create a personal individual account
    INSERT INTO core.accounts (name, slug, account_type, status, modified_by)
    VALUES (
      split_part(NEW.email, '@', 1),
      NEW.id::text,              -- use auth user id as slug (unique)
      'individual',
      'active',
      'domain_trigger'
    )
    RETURNING id INTO v_account_id;

    INSERT INTO core.profile_accounts (profile_id, account_id, role, assigned_by)
    VALUES (NEW.id, v_account_id, 'account_admin', 'domain_trigger');
  END IF;

  RETURN NEW;
END;
$$;

CREATE TRIGGER assign_account_by_domain_trigger
  AFTER INSERT ON auth.users
  FOR EACH ROW EXECUTE FUNCTION core.assign_account_by_domain();
```

Individual accounts are created automatically on signup — no invitation flow needed.

---

## 5. Three-Tier Role Model

```typescript
// packages/shared/src/types/auth.ts

export type UserRole = 'platform_admin' | 'account_admin' | 'user'

export interface SessionCallerContext {
  accountId: string
  userId: string
  role: UserRole
}

/** Build a DB filter based on caller's role.
 *  platform_admin → no filter (all rows)
 *  account_admin  → filter by accountId
 *  user           → filter by accountId + userId
 */
export function callerFilter(
  caller: SessionCallerContext
): Record<string, unknown> {
  if (caller.role === 'platform_admin') return {}
  if (caller.role === 'account_admin') return { accountId: caller.accountId }
  return { accountId: caller.accountId, userId: caller.userId }
}
```

Role enforcement happens at the **repository layer**, not at RLS. Queries are always filtered by `callerFilter(caller)` applied to the WHERE clause. RLS is disabled on account-partitioned tables — partition isolation + application-layer filtering is the security model.

---

## 6. Account-Scoped Repository Pattern

```typescript
// packages/shared/src/repository/factory.ts

export interface AccountRepository {
  repos:       RepoRepository
  sessions:    SessionRepository
  snapshots:   SnapshotRepository
  events:      EventRepository
  apiRequests: ApiRequestRepository
  symbols:     SymbolRepository
  // ... other account-scoped tables
}

export interface AdminRepository {
  accounts:    AccountAdminRepository
  accountKeys: AccountKeyRepository
}

export function createAccountRepository(
  adapter: BackendAdapter,
  accountId: string
): AccountRepository {
  return {
    repos:       new RepoEntity(adapter, accountId),
    sessions:    new SessionEntity(adapter, accountId),
    snapshots:   new SnapshotEntity(adapter, accountId),
    events:      new EventEntity(adapter, accountId),
    apiRequests: new ApiRequestEntity(adapter, accountId),
    symbols:     new SymbolEntity(adapter, accountId),
  }
}
```

Each entity class bakes `accountId` in at construction and injects it into every query:

```typescript
// packages/shared/src/repository/session.ts

export class SessionEntity implements SessionRepository {
  constructor(
    private readonly adapter: BackendAdapter,
    private readonly accountId: string
  ) {}

  async list(caller: SessionCallerContext, opts?: ListOpts): Promise<Session[]> {
    const filter = { ...callerFilter(caller) }
    if (opts?.status) filter.status = opts.status
    const result = await this.adapter.getMany<Session>('sessions', {
      filter,
      orderBy: { column: 'created_at', ascending: false },
      limit: opts?.limit ?? 50
    })
    return result.data ?? []
  }

  async get(id: string, caller: SessionCallerContext): Promise<Session | null> {
    const result = await this.adapter.getOne<Session>('sessions', {
      id,
      ...callerFilter(caller)
    })
    return result.data ?? null
  }

  async create(args: CreateSessionArgs): Promise<string> {
    const id = randomUUID()
    await this.adapter.insert('sessions', {
      id,
      accountId: this.accountId,  // always injected
      ...args
    })
    return id
  }
}
```

---

## 7. Identity & Auth Layer

### 7.1 Central Auth Flow

```
Developer                     Cloud Platform             Supabase Auth
────────                      ──────────────             ─────────────
sensei login                  POST /auth/token
  ↓ GitHub OAuth or email  ──────────────────────►      OAuth exchange
                              ◄──────────────────────   JWT (user_id, email)
                              resolve account_id + role
                              (profile_accounts lookup)
  ◄─── .sensei/credentials ──
  (account_id, user_id, token stored locally)

On every sync call:
  sensei collector reads .sensei/credentials
  → attaches X-Account-Id, X-User-Id, X-User-Role headers
  → POST /sync with session data
```

### 7.2 CLI: `sensei login`

```typescript
// packages/cli/src/commands/login.ts

export async function loginCommand() {
  const { token, userId, accountId, role } = await browserOAuthFlow()

  const verified = await fetch(`${PLATFORM_API}/auth/verify`, {
    headers: { Authorization: `Bearer ${token}` }
  }).then(r => r.json())

  await writeCredentials({
    token,
    userId: verified.userId,
    accountId: verified.accountId,
    accountType: verified.accountType,   // 'individual' | 'team'
    role: verified.role,
    expiresAt: verified.expiresAt
  })

  console.log(`✓ Logged in as ${verified.email} (${verified.accountSlug})`)
}
```

### 7.3 API Auth Middleware

```typescript
// packages/platform/src/middleware/auth.ts  (Hono)

export const authMiddleware = createMiddleware(async (c, next) => {
  const token = c.req.header('authorization')?.slice(7)
  if (!token) return c.json({ error: 'UNAUTHORIZED' }, 401)

  const { data: user, error } = await supabase.auth.getUser(token)
  if (error || !user) return c.json({ error: 'UNAUTHORIZED' }, 401)

  const profile = await adminRepo.accounts.getProfileAccount(user.user.id)
  if (!profile) return c.json({ error: 'NO_ACCOUNT' }, 403)

  c.set('caller', {
    accountId: profile.accountId,
    userId: user.user.id,
    role: profile.role
  } satisfies SessionCallerContext)

  await next()
})
```

---

## 8. Data Sync Pipeline

### 8.1 What Gets Synced

The local collector runs after each `checkpoint()` call and syncs a **scrubbed session summary**:

```typescript
interface SyncPayload {
  sessionId: string
  repoSlug: string            // SHA-256 hash of repo path, account-salted
  startedAt: number
  durationMs: number
  outcome: 'completed' | 'blocked' | 'partial'
  ftrScore: number
  snapshotCount: number
  tokenCost: number
  cacheHitRate: number
  toolCallCount: number
  toolErrorRate: number
  stack: string               // e.g. 'typescript'
  // NOT included: file paths, symbol names, code content, usernames
}
```

### 8.2 PII Scrubbing Rules

| Field | Action |
|-------|--------|
| File paths | Dropped |
| Symbol names | Dropped |
| Code snippets | Dropped |
| Repo name | Hashed (SHA-256, account-specific salt) → `repoSlug` |
| User name | Never collected locally |
| Error messages | Dropped |
| Stack (typescript/python) | Kept — low-cardinality label |

### 8.3 Sync API Endpoint

```
POST /sync/session
Authorization: Bearer <token>
X-Account-Id: <uuid>

Body: SyncPayload[]   (batch up to 100)
```

The ingest handler:
1. Validates JWT → resolves `caller`
2. Asserts `caller.accountId === header X-Account-Id` (no cross-account injection)
3. Upserts into `sessions` partition for `caller.accountId`
4. Returns `{ accepted: N }`

### 8.4 Sync Trigger in Collector

```typescript
// packages/collector/src/sync.ts

export async function syncToCloud(session: LocalSession): Promise<void> {
  const creds = await readCredentials()
  if (!creds) return  // not logged in → silent skip, local-only mode

  const payload = scrub(session)
  await fetch(`${creds.platformUrl}/sync/session`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${creds.token}`,
      'Content-Type': 'application/json',
      'X-Account-Id': creds.accountId
    },
    body: JSON.stringify([payload])
  })
}
```

---

## 9. Platform Account — Cross-Account Analytics

The platform account (`is_platform = true`, `account_type = 'platform'`) has no real users or repos. Its purpose is cross-account aggregate views for platform admins.

### 9.1 Cross-Account Aggregate Views

```sql
-- Global FTR distribution (anonymized, all non-platform accounts)
CREATE VIEW platform.ftr_distribution AS
SELECT
  width_bucket(ftr_score, 0, 1, 10) * 10 AS bucket_pct
, count(*)                                AS session_count
FROM sessions
WHERE account_id != (SELECT id FROM core.accounts WHERE is_platform = true)
GROUP BY 1
ORDER BY 1;

-- Per-account aggregates (identified by slug only, no names/emails)
CREATE VIEW platform.account_stats AS
SELECT
  a.slug                              AS account_slug
, a.account_type
, count(DISTINCT r.id)               AS repo_count
, count(DISTINCT s.id)               AS session_count_30d
, avg(s.ftr_score)                   AS avg_ftr
, avg(s.token_cost)                  AS avg_cost
, sum(s.token_cost)                  AS total_cost_30d
FROM core.accounts a
LEFT JOIN repos r     ON r.account_id = a.id
LEFT JOIN sessions s  ON s.account_id = a.id
  AND s.created_at > extract(epoch FROM now() - INTERVAL '30 days') * 1000
WHERE a.is_platform = false
GROUP BY a.slug, a.account_type;
```

### 9.2 Platform Admin Access Control

Two distinct repository types — never mix them:

- `AccountRepository` — scoped to one account, all queries auto-filtered by `accountId`
- `PlatformRepository` — aggregate read models, accessible only when `caller.role === 'platform_admin'`

```typescript
export function createPlatformRepository(adapter: BackendAdapter): PlatformRepository {
  return {
    accountStats:    new AccountStatsView(adapter),
    ftrDistribution: new FtrDistributionView(adapter),
    toolUsage:       new ToolUsageView(adapter),
  }
}

app.get('/platform/stats', authMiddleware, async (c) => {
  const caller = c.get('caller')
  if (caller.role !== 'platform_admin') return c.json({ error: 'FORBIDDEN' }, 403)
  return c.json(await createPlatformRepository(getAdapter()).accountStats.list())
})
```

---

## 10. Encryption for Sensitive Data

### 10.1 What Gets Encrypted

- LLM API keys stored per account
- OAuth tokens in credentials store

### 10.2 Two-Layer Key Hierarchy

```
SENSEI_KEK (env var, 32 bytes)
  ↓ AES-256-GCM wraps
core.account_keys.encrypted_dek  (per-account DEK, 60 bytes: IV + auth tag + ciphertext)
  ↓ AES-256-GCM encrypts
api_keys.encrypted_value  (stored ciphertext in DB)
```

```typescript
// packages/shared/src/crypto.ts

import { randomBytes, createCipheriv, createDecipheriv } from 'node:crypto'

const ALGO = 'aes-256-gcm'

export function generateDek(): Buffer { return randomBytes(32) }

export function wrapDek(dek: Buffer, kek: Buffer): Buffer {
  const iv = randomBytes(12)
  const cipher = createCipheriv(ALGO, kek, iv)
  const ct = Buffer.concat([cipher.update(dek), cipher.final()])
  return Buffer.concat([iv, cipher.getAuthTag(), ct])  // 60 bytes
}

export function unwrapDek(wrapped: Buffer, kek: Buffer): Buffer {
  const iv = wrapped.subarray(0, 12)
  const tag = wrapped.subarray(12, 28)
  const ct = wrapped.subarray(28)
  const decipher = createDecipheriv(ALGO, kek, iv)
  decipher.setAuthTag(tag)
  return Buffer.concat([decipher.update(ct), decipher.final()])
}

export function encryptSecret(plaintext: string, dek: Buffer): Buffer {
  const iv = randomBytes(12)
  const cipher = createCipheriv(ALGO, dek, iv)
  const ct = Buffer.concat([cipher.update(Buffer.from(plaintext, 'utf8')), cipher.final()])
  return Buffer.concat([iv, cipher.getAuthTag(), ct])
}

export function decryptSecret(encrypted: Buffer, dek: Buffer): string {
  const iv = encrypted.subarray(0, 12)
  const tag = encrypted.subarray(12, 28)
  const ct = encrypted.subarray(28)
  const decipher = createDecipheriv(ALGO, dek, iv)
  decipher.setAuthTag(tag)
  return Buffer.concat([decipher.update(ct), decipher.final()]).toString('utf8')
}
```

Account DEK is provisioned at account creation time:

```typescript
async function provisionAccount(
  name: string,
  slug: string,
  accountType: 'individual' | 'team'
): Promise<string> {
  const kek = Buffer.from(process.env.SENSEI_KEK!, 'hex')
  const dek = generateDek()
  const encryptedDek = wrapDek(dek, kek)

  const accountId = await adminRepo.accounts.create({ name, slug, accountType })
  await adminRepo.accountKeys.provision(accountId, encryptedDek)
  return accountId
}
```

---

## 11. Dashboard — Multi-Account SvelteKit App

### 11.1 Auth Guard

```typescript
// apps/dashboard/src/hooks.server.ts

export const handle: Handle = async ({ event, resolve }) => {
  const token = event.cookies.get('sb-token')
  if (!token && !event.url.pathname.startsWith('/auth')) {
    return redirect(302, '/auth/login')
  }

  if (token) {
    const { data: { user } } = await supabase.auth.getUser(token)
    const profile = user ? await getProfileAccount(user.id) : null

    event.locals.user = user
    event.locals.accountId   = profile?.accountId ?? null
    event.locals.accountType = profile?.accountType ?? null
    event.locals.role        = profile?.role ?? null
  }

  return resolve(event)
}
```

### 11.2 Route Structure

```
/                            Landing (public)
/auth/login                  Login (GitHub OAuth or email)
/auth/callback               Supabase OAuth callback

/dashboard/                  Developer home (FTR ring, recent sessions)
/dashboard/repos             Repo list + health
/dashboard/sessions          Session history
/dashboard/setup             Local setup wizard

/account/                    Account Admin (account_admin, team accounts only)
/account/team                Team stats + FTR leaderboard
/account/repos               Account repo health
/account/members             Member list + onboarding queue
/account/invitations         Invite by email

/platform/                   Platform Admin (platform_admin only)
/platform/accounts           Anonymized cross-account list
/platform/analytics          FTR distribution, tool usage, cost trends
/platform/accounts/[slug]    Account detail (anonymized)
```

Individual accounts skip `/account/` routes — the leaderboard and invite flow are not relevant for solo users.

### 11.3 Role-Gated Layouts

```typescript
// apps/dashboard/src/routes/account/+layout.server.ts
export const load: LayoutServerLoad = async ({ locals }) => {
  if (locals.role !== 'account_admin') throw redirect(302, '/dashboard')
  if (locals.accountType !== 'team') throw redirect(302, '/dashboard')
  return {}
}

// apps/dashboard/src/routes/platform/+layout.server.ts
export const load: LayoutServerLoad = async ({ locals }) => {
  if (locals.role !== 'platform_admin') throw redirect(302, '/dashboard')
  return { accountStats: await platformRepo.accountStats.list() }
}
```

---

## 12. New Package: `packages/platform`

```
packages/platform/
  src/
    repository/
      account.ts         AccountAdminRepository impl
      account-key.ts     AccountKeyRepository impl
      platform-stats.ts  Cross-account read models (platform_admin only)
    middleware/
      auth.ts            JWT validation + caller context
    routes/
      auth.ts            /auth/* handlers
      sync.ts            POST /sync/session
      platform.ts        /platform/* handlers (platform_admin only)
      account.ts         /account/* handlers (account_admin only)
    crypto.ts            KEK/DEK helpers
    provision.ts         Account creation + DEK provisioning
```

---

## 13. Implementation Phases

### Phase A — Account Schema Foundation

**Goal:** `core.accounts`, `core.profile_accounts`, `core.account_keys` tables exist. Tiered partition trigger works — `team` accounts get dedicated partitions, individual accounts fall into the DEFAULT hash-sharded pool. Seed creates platform account + one team account + one individual account.

**Done when:**
```sql
INSERT INTO core.accounts (name, slug, account_type, is_platform, modified_by)
  VALUES ('platform', 'platform', 'platform', true, 'seed');
INSERT INTO core.accounts (name, slug, account_type, domain, modified_by)
  VALUES ('Acme Corp', 'acme', 'team', 'acme.com', 'seed');
INSERT INTO core.accounts (name, slug, account_type, modified_by)
  VALUES ('Jerry', 'jerry', 'individual', 'seed');

-- Acme has dedicated partitions; Jerry is in sessions_individual_N
SELECT tablename FROM pg_tables WHERE tablename LIKE 'sessions_%';
-- → sessions_account_<acme_uuid>, sessions_individual, sessions_individual_0..15
```

**Packages:** `packages/shared` (schema + migration), `database/ddl/`

---

### Phase B — Auth + Login

**Goal:** `sensei login` opens browser, gets token, writes `.sensei/credentials`. Platform API resolves account + role. `sensei whoami` prints identity.

**Done when:**
```bash
sensei login   # opens browser, completes OAuth
sensei whoami  # jerry@acme.com | acme (team) | account_admin
```

**Packages:** `packages/cli` (login, whoami), `packages/platform` (auth routes)

---

### Phase C — Account-Scoped Repository

**Goal:** All data access uses `AccountRepository` factory. No query runs without `account_id` filter.

**Done when:** Integration test with two accounts seeded — `createAccountRepository('acme-uuid').sessions.list(caller)` returns only Acme rows; Acme caller cannot read Jerry's rows even with a direct `get(jerrySessionId)`.

**Packages:** `packages/shared`

---

### Phase D — Data Sync Pipeline

**Goal:** After `checkpoint()`, collector syncs scrubbed session summary to platform API. Dashboard shows session count incrementing.

**Packages:** `packages/collector`, `packages/platform` (sync route)

---

### Phase E — Team Account Admin Views

**Goal:** `account_admin` sees `/account/team` with FTR leaderboard, can invite members by email.

**Done when:** Alice invites Bob → Bob accepts → Bob's sessions appear in Alice's leaderboard.

**Packages:** `apps/dashboard` (account routes), `packages/platform` (invitation routes)

---

### Phase F — Platform Admin Views

**Goal:** `platform_admin` sees anonymized cross-account analytics. Account list shows slug + avg FTR + account type, no names or emails.

**Packages:** `apps/dashboard` (platform routes), `packages/platform` (platform read models)

---

### Phase G — Encryption for API Keys

**Goal:** LLM API keys stored via dashboard are encrypted with the account's DEK. Collector decrypts on read.

**Done when:** Key stored → raw bytea in DB → collector decrypts and calls LLM successfully.

**Packages:** `packages/shared` (crypto), `packages/platform` (key routes), `apps/dashboard` (key settings UI)

---

## 14. What Stays Local

| Data | Rationale |
|------|-----------|
| Source code | Never synced |
| Symbol names + signatures | Never synced |
| File paths | Never synced |
| Error messages | Never synced |
| `.sensei/config.yaml` | Local only — contains API keys |
| Local embeddings | Stay in local Supabase |
| Code patterns (`patterns.md`) | Local only |

The sync pipeline is **additive** — the local system remains fully functional without internet access or a logged-in account.

---

## 15. Security Model Summary

| Layer | Mechanism |
|-------|-----------|
| Auth | Supabase JWT, verified per request |
| Account isolation | Composite PKs `(account_id, id)` + list partitioning |
| Individual isolation | `account_id` column filter within shared DEFAULT partition |
| Row-level access | Application-layer `callerFilter`, not Postgres RLS |
| Sensitive data | AES-256-GCM, KEK (env) → per-account DEK → ciphertext |
| Platform admin data | Separate `PlatformRepository`, `platform_admin` role required |
| Sync pipeline | PII-scrubbed at source; ingest asserts account match |
| Cross-account leakage | Prevented by `accountId` baked into every repository entity |
