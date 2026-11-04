#!/usr/bin/env bash
# Builds Builder's chat client from voipappz/app (public) and puts it in
# client/, which core/build.rs embeds into the binary.
#
#   scripts/client.sh                # the ref in client.ref
#   scripts/client.sh <ref>          # a branch, tag or sha
#   APP_DIR=../app scripts/client.sh # an existing checkout, as it is
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
ref="${1:-$(tr -d '[:space:]' < "$root/client.ref")}"

if [ -n "${APP_DIR:-}" ]; then
  app="$(cd "$APP_DIR" && pwd)"
else
  app="$root/.client-app"
  if [ ! -d "$app/.git" ]; then
    git clone --quiet --filter=blob:none https://github.com/voipappz/app.git "$app"
  fi
  git -C "$app" fetch --quiet origin "$ref"
  git -C "$app" checkout --quiet --detach FETCH_HEAD
  echo "voipappz/app at $(git -C "$app" rev-parse --short HEAD) ($ref)"
fi

(cd "$app" && npm ci --no-audit --no-fund && npm run build:builder)
rm -rf "$root/client"
cp -r "$app/dist-builder" "$root/client"
ls -l "$root/client"
