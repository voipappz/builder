#!/usr/bin/env bash
# End to end, the Mobile platform as a user opens it: Builder started with
# only the phone project (phone/), which runs its make setup and make dev
# (Android emulator, the app installed and opened, the screen stream); then
# the Builder page in Chromium: the chat loads and the live phone shows in
# the pane. Needs KVM, the chat built into client/ (npm run build in chat/)
# before the CLI is built, and NODE_PATH with playwright (chat/node_modules).
#
#   scripts/e2e-phone.sh [work-dir]
set -euo pipefail

work=$(realpath -m "${1:-${RUNNER_TEMP:-/tmp}/builder-e2e-phone}")
root=$(cd "$(dirname "$0")/.." && pwd)
bin=${BUILDER_BIN:-$root/target/debug/voipappz-builder}
port=${BUILDER_PORT:-9832}
log=$work/builder.log
state=${XDG_STATE_HOME:-$HOME/.local/state}/voipappz-builder

[ -f "$root/client/client.js" ] || { echo "✕ no chat in client/; run npm run build in chat/ first"; exit 1; }
[ -n "${BUILDER_BIN:-}" ] || (cd "$root" && cargo build --locked -p builder-cli)
rm -rf "$work" && mkdir -p "$work"

echo "== Start Builder with only the phone project"
"$bin" --bot-dir "$root/phone" --api-url "${VOIPAPPZ_API_URL:-https://switch.voipappz.io}" --port "$port" \
  --url-file "$work/url" >"$log" 2>&1 &
builder=$!
stop() { kill -INT "$builder" 2>/dev/null || true; wait "$builder" 2>/dev/null || true; }
fail() {
  echo "✕ $1"
  echo "--- builder.log"; sed "s/secret=[0-9a-f]*/secret=…/" "$log" | grep -v -i password | tail -n 60
  for f in "$state"/bot-*.log; do [ -f "$f" ] && { echo "--- $(basename "$f") (tail)"; tail -n 60 "$f"; }; done
  [ -f "$root/phone/build/emulator.log" ] && { echo "--- emulator.log (tail)"; tail -n 20 "$root/phone/build/emulator.log"; }
  stop; exit 1
}
trap stop EXIT

# The first make setup downloads the emulator image and builds the stream tool.
deadline=$((SECONDS + 30 * 60))
until [ -s "$work/url" ]; do
  kill -0 "$builder" 2>/dev/null || fail "Builder exited before it was ready"
  [ $SECONDS -lt $deadline ] || fail "Builder was not ready within 30 minutes"
  sleep 5
done
echo "✓ Builder ran the phone's setup and make dev, and its stream answers"

echo "== The page in a browser"
node "$root/scripts/e2e-phone-page.cjs" "$(cat "$work/url")" "$work/builder.png" \
  || fail "the Builder page check failed (screenshot: $work/builder.png)"

echo "== Stopping Builder stops the emulator"
stop; trap - EXIT
for _ in $(seq 1 30); do pgrep -f "qemu-system.*-avd voipappz" >/dev/null || break; sleep 2; done
pgrep -f "qemu-system.*-avd voipappz" >/dev/null && { echo "✕ the emulator still runs after Builder stopped"; exit 1; }
echo "✓ End to end: phone project → setup/dev → Builder (chat) → the live phone in the pane → stop"
