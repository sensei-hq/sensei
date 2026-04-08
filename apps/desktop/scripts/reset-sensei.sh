#!/usr/bin/env bash
# reset-sensei.sh — clear all Sensei desktop state so you can test first-launch
set -euo pipefail

APP_ID="dev.sensei.desktop"

echo "Resetting Sensei desktop state…"

# ── 1. Clear localStorage (WebKit SQLite) ─────────────────────────────────────
# WebKit stores localStorage in a SQLite db per origin.
LS_DIR="$HOME/Library/WebKit/$APP_ID/WebsiteData/Default"

if [ -d "$LS_DIR" ]; then
  DB=$(find "$LS_DIR" -name "localstorage.sqlite3" 2>/dev/null | head -1)
  if [ -n "$DB" ] && command -v sqlite3 &>/dev/null; then
    echo "  Clearing localStorage keys in: $DB"
    sqlite3 "$DB" "DELETE FROM ItemTable WHERE key LIKE 'sensei:%';" 2>/dev/null || true
    # Remove WAL/SHM so WebKit doesn't replay stale transactions
    rm -f "${DB}-wal" "${DB}-shm"
    echo "  ✓ localStorage cleared"
  else
    echo "  ⚠ No localStorage DB found (or sqlite3 not available)"
  fi
else
  echo "  ℹ WebKit data dir not found — app may not have been launched yet"
fi

# ── 2. Clear Tauri app data (IndexedDB, cookies, cache) ───────────────────────
APP_DATA="$HOME/Library/Application Support/$APP_ID"
if [ -d "$APP_DATA" ]; then
  echo "  Clearing app data: $APP_DATA"
  rm -rf "$APP_DATA"
  echo "  ✓ App data cleared"
fi

# ── 3. Clear Tauri cache ──────────────────────────────────────────────────────
APP_CACHE="$HOME/Library/Caches/$APP_ID"
if [ -d "$APP_CACHE" ]; then
  echo "  Clearing cache: $APP_CACHE"
  rm -rf "$APP_CACHE"
  echo "  ✓ Cache cleared"
fi

# ── 4. Remove sensei global DB (optional, destructive) ───────────────────────
SENSEI_DB="$HOME/.sensei/sensei.db"
if [ -f "$SENSEI_DB" ]; then
  read -rp "  Remove $SENSEI_DB? [y/N] " yn
  if [[ "$yn" =~ ^[Yy]$ ]]; then
    rm -f "$SENSEI_DB"
    echo "  ✓ sensei.db removed"
  else
    echo "  ↷ Skipped sensei.db"
  fi
fi

echo ""
echo "Done. Launch the Sensei app to start fresh."
