#!/usr/bin/env bash
# scripts/create-test-user.sh
# Creates (or re-creates) the local dev test user and links them to the seed account.
# Run once after every dbd reset, or whenever the Supabase auth DB is wiped.
#
# Usage: bash scripts/create-test-user.sh

set -euo pipefail

API_URL="http://127.0.0.1:54321"
SERVICE_KEY="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU"
DB_URL="postgresql://postgres:postgres@127.0.0.1:54322/postgres"
ACCOUNT_ID="00000002-0000-0000-0000-000000000001"
EMAIL="dev@sensei.local"
PASSWORD="sensei-dev-2026"

echo "→ Creating or fetching test user: $EMAIL"

RESPONSE=$(curl -s -X POST "$API_URL/auth/v1/admin/users" \
  -H "Authorization: Bearer $SERVICE_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"email_confirm\":true}")

USER_ID=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id',''))" 2>/dev/null)

# If create failed (email_exists), fetch the existing user's ID
if [ -z "$USER_ID" ]; then
  LIST_RESPONSE=$(curl -s "$API_URL/auth/v1/admin/users" \
    -H "Authorization: Bearer $SERVICE_KEY")
  USER_ID=$(echo "$LIST_RESPONSE" | python3 -c "
import sys,json
users=json.load(sys.stdin).get('users',[])
match=[u for u in users if u.get('email')=='$EMAIL']
print(match[0]['id'] if match else '')
" 2>/dev/null)
fi

if [ -z "$USER_ID" ]; then
  echo "✗ Could not create or find user $EMAIL"
  exit 1
fi

echo "  user_id: $USER_ID"

echo "→ Linking user to account $ACCOUNT_ID as owner"
psql "$DB_URL" -q << SQL
INSERT INTO core.profile_accounts (user_id, account_id, role)
VALUES ('$USER_ID', '$ACCOUNT_ID', 'owner')
ON CONFLICT (user_id, account_id) DO NOTHING;
SQL

echo "✓ Done. Sign in at http://localhost:5173/auth with:"
echo "  email:    $EMAIL"
echo "  password: $PASSWORD"
