#!/usr/bin/env bash
# End to end on the appz, the way a user runs Builder since v0.1.19: only the
# customer's Elixir appz folder (Connectix), no React app. Clones Connectix,
# starts the Builder CLI with that folder (Connectix's `make setup` and
# `make dev` in Docker), then checks in a real browser that the built-in
# chat renders, the appz's tab shows its Calls dashboard, and the agent
# answers. No chat turn: that needs a model key.
#
#   scripts/e2e-appz.sh [work-dir]
#
# CONNECTIX_DIR uses an existing checkout instead of cloning; CONNECTIX_REF
# picks the branch (default main). The chat client must be in client/
# (scripts/client.sh) before the CLI is built.
set -euo pipefail

work=$(realpath -m "${1:-${RUNNER_TEMP:-/tmp}/builder-e2e-appz}")
root=$(cd "$(dirname "$0")/.." && pwd)
bin=${BUILDER_BIN:-$root/target/debug/voipappz-builder}
api_url=${VOIPAPPZ_API_URL:-https://switch.voipappz.io}
port=${BUILDER_PORT:-9832}
log=$work/builder.log
state=${XDG_STATE_HOME:-$HOME/.local/state}/voipappz-builder

[ -f "$root/client/client.js" ] || { echo "✕ no chat client in client/; run scripts/client.sh first"; exit 1; }
[ -n "${BUILDER_BIN:-}" ] || (cd "$root" && cargo build --locked -p builder-cli)

rm -rf "$work" && mkdir -p "$work"
if [ -n "${CONNECTIX_DIR:-}" ]; then
  appz=$(realpath "$CONNECTIX_DIR")
else
  appz=$work/connectix
  echo "== Clone connectix-io/connectix (${CONNECTIX_REF:-main})"
  git clone --quiet --depth 1 --branch "${CONNECTIX_REF:-main}" git@github.com:connectix-io/connectix.git "$appz"
fi
[ -f "$appz/builder.toml" ] || { echo "✕ $appz has no builder.toml"; exit 1; }

echo "== Start Builder with only the appz folder"
"$bin" --bot-dir "$appz" --api-url "$api_url" --port "$port" \
  --url-file "$work/url" >"$log" 2>&1 &
builder=$!
stop() { kill -INT "$builder" 2>/dev/null || true; wait "$builder" 2>/dev/null || true; }
fail() {
  echo "✕ $1"
  echo "--- builder.log"; sed "s/secret=[0-9a-f]*/secret=…/" "$log" | grep -v -i password | tail -n 80
  for f in "$state"/bot-*.log; do [ -f "$f" ] && { echo "--- $(basename "$f") (tail)"; tail -n 60 "$f"; }; done
  stop; exit 1
}
trap stop EXIT

# The first `make setup` builds Connectix's dev image.
deadline=$((SECONDS + 40 * 60))
until [ -s "$work/url" ]; do
  kill -0 "$builder" 2>/dev/null || fail "Builder exited before it was ready"
  [ $SECONDS -lt $deadline ] || fail "Builder was not ready within 40 minutes"
  sleep 5
done
sed -e 's/^/  /' -e 's/secret=[0-9a-f]*/secret=…/' "$log" | grep -v -i password | grep -v "Waiting for the product" || true

echo "== Appz mode"
grep -q "No local app: the chat is built into Builder." "$log" || fail "Builder did not start in appz-only mode"
grep -q "make dev PORT=" "$log" && ! grep -q "Starting the app (make dev" "$log" || fail "Builder started a React app"
echo "✓ No React app; the appz runs"

echo "== The page in a browser"
node "$root/scripts/e2e-appz-page.cjs" "$(cat "$work/url")" "$work/builder.png" || fail "the Builder page check failed (screenshot: $work/builder.png)"

echo "== Agent (ACP initialize, in the appz folder)"
cd "$appz"
reply=$( { echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}'; sleep 30; } \
  | timeout 90 npx --yes @agentclientprotocol/claude-agent-acp@0.94.0 2>"$work/agent.err" | head -n 1 || true)
echo "$reply" | grep -q '"protocolVersion"' || { cat "$work/agent.err"; fail "the agent did not answer initialize"; }
echo "✓ The agent answers initialize"

echo "✓ End to end: appz folder → make setup/dev → Builder (built-in chat) → appz tab → agent"
