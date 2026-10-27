#!/usr/bin/env bash
# End to end, the way a user starts Builder: clone voipappz/app, let the
# Builder CLI make its worktree, run the app's `make setup` and `make dev`,
# then check the Builder page answers and the agent starts (ACP initialize).
# No chat turn: that needs a model key.
#
#   TEST_EMAIL=… TEST_PASSWORD=… scripts/e2e.sh [work-dir]
#
# APP_REF picks the app branch (default main). The CLI binary comes from
# `cargo build`; set BUILDER_BIN to use another one.
set -euo pipefail

: "${TEST_EMAIL:?the app's make setup signs in with TEST_EMAIL and TEST_PASSWORD}"
: "${TEST_PASSWORD:?the app's make setup signs in with TEST_EMAIL and TEST_PASSWORD}"
work=$(realpath -m "${1:-${RUNNER_TEMP:-/tmp}/builder-e2e}")
app_ref=${APP_REF:-main}
api_url=${VOIPAPPZ_API_URL:-https://switch.voipappz.io}
port=${BUILDER_PORT:-9832}
root=$(cd "$(dirname "$0")/.." && pwd)
bin=${BUILDER_BIN:-$root/target/debug/voipappz-builder}
log=$work/builder.log
state=${XDG_STATE_HOME:-$HOME/.local/state}/voipappz-builder

[ -n "${BUILDER_BIN:-}" ] || (cd "$root" && cargo build --locked -p builder-cli)

echo "== Clone voipappz/app ($app_ref) into $work/app"
rm -rf "$work" && mkdir -p "$work"
git clone --quiet --branch "$app_ref" https://github.com/voipappz/app.git "$work/app"

echo "== Start Builder (make setup, make dev)"
"$bin" --app-repo "$work/app" --start-app --api-url "$api_url" --port "$port" \
  --url-file "$work/url" >"$log" 2>&1 &
builder=$!
stop() { kill "$builder" 2>/dev/null || true; wait "$builder" 2>/dev/null || true; }
fail() {
  echo "✕ $1"
  echo "--- builder.log"; sed "s/secret=[0-9a-f]*/secret=…/" "$log"
  [ -f "$state/app.log" ] && { echo "--- app.log (tail)"; tail -n 60 "$state/app.log"; }
  stop; exit 1
}
trap stop EXIT

deadline=$((SECONDS + 25 * 60))
until [ -s "$work/url" ]; do
  kill -0 "$builder" 2>/dev/null || fail "Builder exited before it was ready"
  [ $SECONDS -lt $deadline ] || fail "Builder was not ready within 25 minutes"
  sleep 5
done
sed -e 's/^/  /' -e 's/secret=[0-9a-f]*/secret=…/' "$log" | grep -v -i password || true

echo "== Builder page"
url=$(cat "$work/url")
status=$(curl -s -o /dev/null -w '%{http_code}' "$url")
[ "$status" = 200 ] || fail "Builder page answered $status"
echo "✓ Builder page answers"

echo "== App dev server"
app_port=$(sed -n 's/.*make dev PORT=\([0-9]*\).*/\1/p' "$log" | head -n 1)
[ -n "$app_port" ] || fail "Builder did not start make dev"
curl -sf -o /dev/null "http://localhost:$app_port/" || fail "the app does not answer on port $app_port"
echo "✓ The app answers on port $app_port"

echo "== Agent (ACP initialize)"
# The command Builder picks: the app's own package, or npx until it has one.
cd "$work/app-builder"
if [ -e node_modules/.bin/claude-agent-acp ]; then agent=(bin/npm.sh exec -- claude-agent-acp)
else agent=(npx --yes @agentclientprotocol/claude-agent-acp@0.94.0); fi
echo "  ${agent[*]}"
reply=$( { echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}'; sleep 30; } \
  | timeout 60 "${agent[@]}" 2>"$work/agent.err" | head -n 1 || true)
echo "$reply" | grep -q '"protocolVersion"' || { cat "$work/agent.err"; fail "the agent did not answer initialize"; }
echo "✓ The agent answers initialize"

echo "✓ End to end: clone → setup → dev → Builder → agent"
