---
title: Shared credential-vault crate — blueprint (cross-repo, joint)
description: Extract strategos' battle-tested-but-inline BYOK vault into a shared Rust crate in sensei-hq/gateway that both the strategos gateway and the sensei daemon depend on — closing the three security gaps (KMS-backed KEK, tenant‖router AAD binding, key rotation) once, for both. Execution handed to the strategos session via sensei-hq/gateway issues.
type: blueprint
status: blueprint
created: 2026-07-27
depends_on:
  - docs/blueprints/2026-07-27-dojo-full-surface.md
references:
  - ~/Developer/strategos/monorepo/services/gateway/src/crypto.rs
  - ~/Developer/strategos/monorepo/services/gateway/src/vault.rs
  - ~/Developer/strategos/monorepo/services/gateway/src/state.rs
  - ~/Developer/strategos/monorepo/database/policies/secrets.sql
  - sensei-hq/gateway (the shared crate both consume)
  - sensei-hq/torii#16 (OAuth follow-up) · sensei-hq/torii#17 (KMS KEK follow-up)
---

# Shared credential-vault crate — blueprint (cross-repo, joint)

## Objective

The BYOK vault that guards provider API keys is **security-critical and wanted in two places**
(strategos gateway + sensei dōjō). Rather than copy it, **extract it into one hardened, shared crate**
in **`sensei-hq/gateway`** (already *"Used by Sensei & Strategos"*) that both depend on — and close
the three gaps strategos left open **once, for both**. This is literally the `torii-vault` crate the
strategos P4 identity/keyvault plan specified but never built (it shipped inline in the gateway
service instead). Execution is **handed to the parallel strategos session** via `sensei-hq/gateway`
issues.

## Current reality (strategos, built)

Solid core, three gaps. ~250 lines inline in `services/gateway/src/{crypto,vault,state}.rs`:

- **Envelope crypto** — AES-256-GCM (RustCrypto, not hand-rolled). Per-tenant **DEK** sealed under a
  **KEK**; provider keys sealed under the DEK. Layout `[12B IV][16B tag][ct]`. All plaintext in
  `Zeroizing`; tamper → fail-closed.
- **Vault** — `store/rotate/revoke/resolve_router_key`, `ensure_tenant_dek`; the gateway
  (`service_role`) is the **only** decryptor; keys never returned/logged.
- **Storage** — `core.tenant_keys(encrypted_dek)` + `public.router_credentials(encrypted_api_key,
  is_active, credential_type)`; **RLS deny-all, service_role-only** (`secrets.sql`).
- **Runtime** — `TenantKeyCache` (decrypt-on-miss, `invalidate` on write).
- **Per-call seam (already crate-level, reuse free)** — `InferenceRequest.credentials:
  HashMap<String,String>` (redacting `Debug`); engine overrides the dispatched router's `api_key`.

**The three gaps (all crate-level):**
1. **KEK lives in an env var** (`TORII_KEK`), not a KMS/HSM — biggest deviation. (tracked: torii#17)
2. **No AAD binding** — a sealed blob isn't bound to its `(tenant, router)`; a DB-write actor could
   relocate ciphertext across rows.
3. **No rotation** — `dek_version` exists but there's no `rotate_dek`/`rotate_kek`/archive; "rotate"
   overwrites the row in place (no zero-downtime).
(Also: drop the still-live HS256 JWT fallback; add per-use decrypt audit.)

## Target — the shared crate

**Crate `sensei-vault`** at `crates/vault/` in `sensei-hq/gateway` (core + a `sqlx` storage feature).
Filed as epic **[sensei-hq/gateway#38](https://github.com/sensei-hq/gateway/issues/38)** (V1–V6
checkboxes) for the strategos session. Public surface:

```
crypto:   seal/unseal_credential(dek, aad, pt) · seal/unseal_dek(kek, dek) · generate_dek()
KekProvider (trait):  KmsKekProvider (prod) · EnvKekProvider (dev, FAILS CLOSED under a prod profile)
VaultStore (trait):   store/rotate/revoke/resolve/ensure_dek  — Postgres adapter impl provided
Vault<K: KekProvider, S: VaultStore>:  store/rotate/revoke/resolve_router_key
TenantKeyCache:  get(tenant) -> Arc<{router:key}> · invalidate(tenant)
```

Design changes that close the gaps:
1. **`KekProvider` trait** — KEK bytes never sit raw in process env in prod; `EnvKekProvider` is
   dev-only and refuses to load under a prod profile. (Aligns torii#17.)
2. **AAD binding** — `seal/unseal_credential` take `aad = tenant_id ‖ router_id`; a relocated blob
   fails AEAD auth. (Rotation-migration: re-seal existing rows with AAD.)
3. **Rotation** — `rotate_dek` (+ a `*_key_archive` for the old DEK) and `rotate_kek` (re-wrap DEKs
   only); relax `UNIQUE(tenant,router)` to allow a brief rotation overlap.

**Invariants (non-negotiable):** decrypt only in the trusted process (sensei **daemon** / strategos
**gateway**), never in a Worker/client; RLS deny-all + service_role-only; keys never returned or
logged; value-redacting `Debug`; `Zeroizing` throughout; fail-closed on tamper/missing-KEK.

## Consumers

- **strategos gateway** — migrate the inline `crypto.rs`/`vault.rs`/`TenantKeyCache` onto the crate
  (behaviour-preserving; the per-call seam is already crate-level). Keeps torii's tables/RLS.
- **sensei daemon** — depend on the crate; supply the Postgres `VaultStore` adapter for sensei's
  schema (`dojo`/`sensei`), a `KekProvider`, and wire `TenantKeyCache` → the `credentials` map on the
  gateway call. Reuse the `secrets.sql` RLS + the two-table shape near-verbatim.

## Decomposition → `sensei-hq/gateway` issues (hand-off)

Filed as one epic — **[sensei-hq/gateway#38](https://github.com/sensei-hq/gateway/issues/38)** —
with V1–V6 as a task list. Ordered; V1/V2 have no deps.

- **V1 · Extract `crypto` into the crate** — `seal/unseal_credential` (now AAD-bound),
  `seal/unseal_dek`, `generate_dek`; `Zeroizing`, `[IV][tag][ct]`, tamper fail-closed. Unit tests
  (round-trip, nonce distinctness, bit-flip tamper, **AAD-mismatch rejection**, short-blob).
- **V2 · `KekProvider` trait + Env/Kms impls** — `EnvKekProvider` (dev, prod-profile fail-closed) +
  `KmsKekProvider` (prod; wraps the platform KMS/Secrets Store). Closes torii#17.
- **V3 · `VaultStore` trait + Postgres adapter + `Vault`** — `ensure_tenant_dek`, store/rotate/
  revoke/resolve; the Postgres impl matches the `tenant_keys`/`router_credentials` DDL; carry the
  `secrets.sql` RLS. Tests: DEK-less auto-provision → round-trip; rotate leaves one active row.
- **V4 · Rotation** — `rotate_dek` (+ DEK archive) and `rotate_kek` (re-wrap only); AAD re-seal
  migration for existing rows; relax the unique constraint for overlap.
- **V5 · `TenantKeyCache` + strategos migration** — move the cache into the crate; migrate the
  strategos gateway off its inline vault onto the crate (behaviour-preserving; suite stays green).
- **V6 · sensei daemon adoption** — sensei depends on the crate, supplies the `VaultStore` adapter +
  `KekProvider`, wires `TenantKeyCache` → `credentials`. (Executed on the sensei plane; the dōjō
  Connections UI in W2 of the surface blueprint consumes the resulting `/v1`/`rpc` routes.)

Cross-reference **torii#16** (OAuth vault — sequel, build fresh from the existing design) and
**torii#17** (KMS KEK — folds into V2). Each issue: `depth:build`, tests + a **Verification** block,
zero-errors gate, never-merge-on-red — matching the strategos F3 plan's rigor.

## Status

Crate name + placement confirmed (`sensei-vault` @ `crates/vault/`, `sensei-hq/gateway`); hand-off
epic filed (gateway#38). This is cross-repo and guards real user keys — **no code until the plan is
picked up by the strategos session**; the sensei-plane task (V6) sequences after the crate lands.
