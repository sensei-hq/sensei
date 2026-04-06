#!/usr/bin/env bash
# scripts/create-test-user.sh
# Creates (or re-creates) all local dev test users and links them to seed accounts/teams.
# Run once after every dbd reset, or whenever the Supabase auth DB is wiped.
#
# Usage: bash scripts/create-test-user.sh
#
# Users created:
#   jerry@senecaglobal.com  → jerry-thomas  (senecaglobal, owner, is_platform account)
#   bob@acme.com            → bob-kim       (acme-corp, member; engineering team)
#   carol@acme.com          → carol-singh   (acme-corp, admin; engineering+product maintainer)
#   alice@acme.com          → alice-chen    (acme-corp, member; product team)
#   dave@devstudio.com      → dave-miller   (devstudio, owner; devops team maintainer)
#   dev@sensei.local        → dev-user      (sensei-dev, owner — legacy MCP account)

set -euo pipefail

API_URL="http://127.0.0.1:54321"
SERVICE_KEY="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU"
DB_URL="postgresql://postgres:postgres@127.0.0.1:54322/postgres"
PASSWORD="sensei-dev-2026"

# ---------------------------------------------------------------------------
# Helper: look up a core.accounts id by slug
# ---------------------------------------------------------------------------
account_id() {
  psql "$DB_URL" -t -c "SELECT id FROM core.accounts WHERE slug = '$1'" | tr -d ' \n'
}

# ---------------------------------------------------------------------------
# Helper: look up a core.teams id by account slug + team slug
# ---------------------------------------------------------------------------
team_id() {
  psql "$DB_URL" -t -c "
    SELECT t.id FROM core.teams t
    JOIN core.accounts a ON a.id = t.account_id
    WHERE a.slug = '$1' AND t.slug = '$2'
  " | tr -d ' \n'
}

# ---------------------------------------------------------------------------
# Helper: create or fetch a Supabase auth user, return UUID
# ---------------------------------------------------------------------------
create_or_fetch_user() {
  local email="$1"
  local display_name="$2"

  local response
  response=$(curl -s -X POST "$API_URL/auth/v1/admin/users" \
    -H "Authorization: Bearer $SERVICE_KEY" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$email\",\"password\":\"$PASSWORD\",\"email_confirm\":true,\"user_metadata\":{\"display_name\":\"$display_name\"}}")

  local uid
  uid=$(echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id',''))" 2>/dev/null)

  # If create failed (duplicate email), fetch existing user ID
  if [ -z "$uid" ]; then
    local list
    list=$(curl -s "$API_URL/auth/v1/admin/users" \
      -H "Authorization: Bearer $SERVICE_KEY")
    uid=$(echo "$list" | python3 -c "
import sys,json
users=json.load(sys.stdin).get('users',[])
match=[u for u in users if u.get('email')=='$email']
print(match[0]['id'] if match else '')
" 2>/dev/null)
  fi

  echo "$uid"
}

# ---------------------------------------------------------------------------
# Helper: insert profile, account membership, and optional team memberships
# Args: user_id slug display_name account_id account_role [team_id:role ...]
# ---------------------------------------------------------------------------
link_user() {
  local user_id="$1"
  local slug="$2"
  local display_name="$3"
  local acct_id="$4"
  local account_role="$5"
  shift 5
  local team_pairs=("$@")

  psql "$DB_URL" -q << SQL
INSERT INTO core.profiles (user_id, slug, display_name)
VALUES ('$user_id', '$slug', '$display_name')
ON CONFLICT (user_id) DO UPDATE SET slug = excluded.slug, display_name = excluded.display_name;

INSERT INTO core.profile_accounts (user_id, account_id, role)
VALUES ('$user_id', '$acct_id', '$account_role')
ON CONFLICT (user_id, account_id) DO UPDATE SET role = excluded.role;
SQL

  if [ "${#team_pairs[@]}" -gt 0 ]; then
    for pair in "${team_pairs[@]}"; do
      local t_id="${pair%%:*}"
      local t_role="${pair##*:}"
      psql "$DB_URL" -q << SQL
INSERT INTO core.team_members (team_id, user_id, role)
VALUES ('$t_id', '$user_id', '$t_role')
ON CONFLICT (team_id, user_id) DO UPDATE SET role = excluded.role;
SQL
    done
  fi
}

# ---------------------------------------------------------------------------
# Resolve account and team IDs from slugs (set after dbd import)
# ---------------------------------------------------------------------------
echo "→ Resolving account and team IDs from database..."

ACCOUNT_SENECAGLOBAL=$(account_id "senecaglobal")
ACCOUNT_ACME=$(account_id "acme-corp")
ACCOUNT_DEVSTUDIO=$(account_id "devstudio")
ACCOUNT_SENSEI_DEV=$(account_id "sensei-dev")

TEAM_ENGINEERING=$(team_id "acme-corp" "engineering")
TEAM_PRODUCT=$(team_id "acme-corp" "product")
TEAM_DEVOPS=$(team_id "devstudio" "devops")
TEAM_PLATFORM_OPS=$(team_id "senecaglobal" "platform-ops")

for var in ACCOUNT_SENECAGLOBAL ACCOUNT_ACME ACCOUNT_DEVSTUDIO ACCOUNT_SENSEI_DEV \
           TEAM_ENGINEERING TEAM_PRODUCT TEAM_DEVOPS TEAM_PLATFORM_OPS; do
  [ -z "${!var}" ] && { echo "✗ Could not resolve $var — run dbd import first"; exit 1; }
done

echo "  accounts and teams resolved ✓"

# ---------------------------------------------------------------------------
# Create users
# ---------------------------------------------------------------------------

echo "→ jerry@senecaglobal.com (jerry-thomas, senecaglobal owner + platform admin)"
UID_JERRY=$(create_or_fetch_user "jerry@senecaglobal.com" "Jerry Thomas")
[ -z "$UID_JERRY" ] && { echo "✗ Failed to create jerry"; exit 1; }
echo "  user_id: $UID_JERRY"
psql "$DB_URL" -q -c "
UPDATE auth.users
SET raw_app_meta_data = raw_app_meta_data || '{\"role\":\"platform_admin\"}'::jsonb
WHERE id = '$UID_JERRY';
"
echo "  app_metadata.role = platform_admin ✓"
link_user "$UID_JERRY" "jerry-thomas" "Jerry Thomas" \
  "$ACCOUNT_SENECAGLOBAL" "owner" \
  "$TEAM_PLATFORM_OPS:maintainer"

echo "→ bob@acme.com (bob-kim, acme-corp member, engineering team)"
UID_BOB=$(create_or_fetch_user "bob@acme.com" "Bob Kim")
[ -z "$UID_BOB" ] && { echo "✗ Failed to create bob"; exit 1; }
echo "  user_id: $UID_BOB"
link_user "$UID_BOB" "bob-kim" "Bob Kim" \
  "$ACCOUNT_ACME" "member" \
  "$TEAM_ENGINEERING:member"

echo "→ carol@acme.com (carol-singh, acme-corp admin, engineering+product maintainer)"
UID_CAROL=$(create_or_fetch_user "carol@acme.com" "Carol Singh")
[ -z "$UID_CAROL" ] && { echo "✗ Failed to create carol"; exit 1; }
echo "  user_id: $UID_CAROL"
link_user "$UID_CAROL" "carol-singh" "Carol Singh" \
  "$ACCOUNT_ACME" "admin" \
  "$TEAM_ENGINEERING:maintainer" \
  "$TEAM_PRODUCT:maintainer"

echo "→ alice@acme.com (alice-chen, acme-corp member, product team)"
UID_ALICE=$(create_or_fetch_user "alice@acme.com" "Alice Chen")
[ -z "$UID_ALICE" ] && { echo "✗ Failed to create alice"; exit 1; }
echo "  user_id: $UID_ALICE"
link_user "$UID_ALICE" "alice-chen" "Alice Chen" \
  "$ACCOUNT_ACME" "member" \
  "$TEAM_PRODUCT:member"

echo "→ dave@devstudio.com (dave-miller, devstudio owner, devops maintainer)"
UID_DAVE=$(create_or_fetch_user "dave@devstudio.com" "Dave Miller")
[ -z "$UID_DAVE" ] && { echo "✗ Failed to create dave"; exit 1; }
echo "  user_id: $UID_DAVE"
link_user "$UID_DAVE" "dave-miller" "Dave Miller" \
  "$ACCOUNT_DEVSTUDIO" "owner" \
  "$TEAM_DEVOPS:maintainer"

echo "→ dev@sensei.local (dev-user, legacy sensei-dev account)"
UID_DEV=$(create_or_fetch_user "dev@sensei.local" "Dev User")
[ -z "$UID_DEV" ] && { echo "✗ Failed to create dev user"; exit 1; }
echo "  user_id: $UID_DEV"
link_user "$UID_DEV" "dev-user" "Dev User" \
  "$ACCOUNT_SENSEI_DEV" "owner"

echo ""
echo "✓ All test users created. Sign in at http://localhost:5173/auth"
echo ""
echo "  jerry@senecaglobal.com  / $PASSWORD  → /jerry-thomas  (platform + team access)"
echo "  bob@acme.com            / $PASSWORD  → /bob-kim        (acme-corp engineering)"
echo "  carol@acme.com          / $PASSWORD  → /carol-singh    (acme-corp admin)"
echo "  alice@acme.com          / $PASSWORD  → /alice-chen     (acme-corp product)"
echo "  dave@devstudio.com      / $PASSWORD  → /dave-miller    (devstudio owner)"
echo "  dev@sensei.local        / $PASSWORD  → /dev-user       (legacy)"
