#!/usr/bin/env bash
set -euo pipefail

# Bump versions across the entire sensei ecosystem:
# - package.json (root + packages/* + apps/*)
# - Cargo.toml (crates/senseid, crates/sensei-mcp, crates/sensei-cli)
# - Homebrew formula + cask (../sensei-homebrew)
#
# Usage:
#   ./scripts/bump-all.sh patch    # 0.1.0 → 0.1.1
#   ./scripts/bump-all.sh minor    # 0.1.0 → 0.2.0
#   ./scripts/bump-all.sh major    # 0.1.0 → 1.0.0
#   ./scripts/bump-all.sh 0.2.0    # explicit version

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOMEBREW_DIR="${REPO_ROOT}/../sensei-homebrew"
MARKETPLACE_DIR="${REPO_ROOT}/../sensei-marketplace"

# ── Determine new version ─────────────────────────────────────────────────────

BUMP_TYPE="${1:-patch}"

# Get current version from root package.json
CURRENT=$(python3 -c "import json; print(json.load(open('${REPO_ROOT}/package.json'))['version'])")
echo "Current version: ${CURRENT}"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP_TYPE" in
  patch) NEW_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
  minor) NEW_VERSION="${MAJOR}.$((MINOR + 1)).0" ;;
  major) NEW_VERSION="$((MAJOR + 1)).0.0" ;;
  *)     NEW_VERSION="$BUMP_TYPE" ;;  # explicit version
esac

echo "New version:     ${NEW_VERSION}"
echo ""

# ── Bump JS packages ─────────────────────────────────────────────────────────

echo "[1/4] Bumping JS packages..."
cd "$REPO_ROOT"

# Root
python3 -c "
import json
for f in ['package.json']:
    d = json.load(open(f))
    d['version'] = '${NEW_VERSION}'
    json.dump(d, open(f, 'w'), indent=2)
    print(f'  {f}: {d[\"version\"]}')
"

# Workspaces
for pkg in packages/*/package.json apps/*/package.json; do
  [ -f "$pkg" ] || continue
  python3 -c "
import json
d = json.load(open('$pkg'))
d['version'] = '${NEW_VERSION}'
json.dump(d, open('$pkg', 'w'), indent=2)
print(f'  $pkg')
" 2>/dev/null || true
done

# ── Bump Rust crates ──────────────────────────────────────────────────────────

echo "[2/4] Bumping Rust crates..."
for toml in crates/*/Cargo.toml; do
  [ -f "$toml" ] || continue
  sed -i '' "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$toml"
  echo "  $toml"
done

# ── Bump Homebrew ─────────────────────────────────────────────────────────────

echo "[3/4] Bumping Homebrew formulas..."
if [ -d "$HOMEBREW_DIR" ]; then
  for rb in "$HOMEBREW_DIR"/Formula/*.rb "$HOMEBREW_DIR"/Casks/*.rb; do
    [ -f "$rb" ] || continue
    sed -i '' "s/version \".*\"/version \"${NEW_VERSION}\"/" "$rb"
    echo "  $(basename "$rb")"
  done
else
  echo "  Skipped (${HOMEBREW_DIR} not found)"
fi

# ── Bump Marketplace ──────────────────────────────────────────────────────────

echo "[4/4] Bumping marketplace..."
if [ -f "$MARKETPLACE_DIR/package.json" ]; then
  python3 -c "
import json
d = json.load(open('${MARKETPLACE_DIR}/package.json'))
d['version'] = '${NEW_VERSION}'
json.dump(d, open('${MARKETPLACE_DIR}/package.json', 'w'), indent=2)
print(f'  package.json')
"
  # Also bump catalog.json
  if [ -f "$MARKETPLACE_DIR/catalog.json" ]; then
    python3 -c "
import json
d = json.load(open('${MARKETPLACE_DIR}/catalog.json'))
d['version'] = '${NEW_VERSION}'
json.dump(d, open('${MARKETPLACE_DIR}/catalog.json', 'w'), indent=2)
print(f'  catalog.json')
"
  fi
else
  echo "  Skipped (${MARKETPLACE_DIR} not found)"
fi

echo ""
echo "Version bumped to ${NEW_VERSION} across all repos."
echo ""
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Commit:  git add -A && git commit -m 'chore: bump version to ${NEW_VERSION}'"
echo "  3. Tag:     git tag v${NEW_VERSION}"
echo "  4. Push:    git push origin develop --tags"
echo "  5. Commit homebrew: cd ../sensei-homebrew && git add -A && git commit -m 'bump ${NEW_VERSION}' && git push"
