#!/bin/sh
# While `make dev` runs (Builder): when a source file of the app changes,
# rebuild, reinstall and reopen it on the emulator, and print the app's log
# (logcat) so it shows in Builder's log. Run from phone/; ADB is the adb
# command with the emulator's serial.
#
#   ADB="mise exec -- adb -s emulator-5554" tools/dev-loop.sh
set -u
: "${ADB:?set ADB}"
stamp=build/.installed
mkdir -p build && touch "$stamp"

# The app's own log lines (and crashes), from now on.
$ADB logcat -c 2>/dev/null
( while :; do
    pid=$($ADB shell pidof io.voipappz.phone 2>/dev/null | tr -d '\r')
    if [ -n "$pid" ]; then $ADB logcat --pid="$pid" -v brief '*:I' 2>/dev/null | sed -u 's/^/[app] /'; fi
    sleep 2
  done ) &

changed() {
  find sip/src shared/src androidApp/src build.gradle.kts settings.gradle.kts gradle/libs.versions.toml \
       sip/build.gradle.kts shared/build.gradle.kts androidApp/build.gradle.kts \
       -type f -newer "$stamp" 2>/dev/null | head -n 1
}

while :; do
  sleep 2
  [ -n "$(changed)" ] || continue
  sleep 1            # let a burst of saves finish
  touch "$stamp"
  echo "[reload] change seen: building and reinstalling the app…"
  start=$(date +%s)
  if make -s install launch ADB="$ADB" >build/reload.log 2>&1; then
    echo "[reload] the app is reinstalled and reopened ($(( $(date +%s) - start )) s)"
  else
    echo "[reload] the build failed; the app on the phone is unchanged:"
    grep -E "^e: |error:|FAILED|What went wrong" -A3 build/reload.log | head -n 30
  fi
done
