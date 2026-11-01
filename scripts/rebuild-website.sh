#!/usr/bin/env bash
# Rebuild sensei-hq.com after a release, and see the new version live (#346).
#
#   scripts/rebuild-website.sh <version>
#
# The website is its own repository (sensei-hq/website); its Sensei page reads this
# repo's VERSION on main when it is built. A release POSTs the site's Cloudflare deploy
# hook ($WEBSITE_DEPLOY_HOOK, a release secret) once main's VERSION reads <version>, then
# waits for the live page to show it. A missing hook, or a site still showing the old version after the polls,
# fails the job — a release never passes with the site quietly out of date.
set -euo pipefail

version="${1:?usage: rebuild-website.sh <version>}"
: "${WEBSITE_DEPLOY_HOOK:?WEBSITE_DEPLOY_HOOK (the Cloudflare deploy hook URL of the website) is not set}"
page="${WEBSITE_URL:-https://sensei-hq.com}/sensei"
polls="${WEBSITE_POLLS:-40}"
pause="${WEBSITE_POLL_SECONDS:-15}"

# GitHub's raw CDN can serve main's old VERSION for minutes after the release; a build
# started then would show the old version. Fire the hook only once main shows this one.
raw="https://raw.githubusercontent.com/sensei-hq/sensei/main/VERSION"
seen=""
for _ in $(seq "$polls"); do
  seen=$(curl -fsS "$raw" | tr -d '[:space:]' || true)
  [[ "$seen" == "$version" ]] && break
  sleep "$pause"
done
if [[ "$seen" != "$version" ]]; then
  echo "rebuild-website: main's VERSION reads ${seen:-nothing}, not $version — the site was not rebuilt" >&2
  exit 1
fi

curl -fsS -X POST "$WEBSITE_DEPLOY_HOOK" > /dev/null

served=""
for _ in $(seq "$polls"); do
  served=$(curl -fsS "$page" | sed -n 's/.*footer-version[^>]*>v\([0-9][0-9.]*\)<.*/\1/p' | head -1 || true)
  if [[ "$served" == "$version" ]]; then
    echo "rebuild-website: $page serves v$version"
    exit 0
  fi
  sleep "$pause"
done
echo "rebuild-website: $page serves v${served:-nothing}, not v$version, after $polls checks" >&2
exit 1
