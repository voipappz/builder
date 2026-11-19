# The targets VoipAppz Builder runs (builder.toml). Builder shows the Android
# emulator's screen, streamed by ws-scrcpy (MIT) on port 8000, in its pane
# beside the chat. Builder passes PORT to every target: it must be 8000.
#
# Java 17 and the Android SDK come from mise (mise.toml): every command runs
# through `mise exec`, so it works with or without mise activated in the shell.
MISE = mise exec --
GRADLE = $(MISE) ./gradlew --console=plain
WS_SCRCPY = .tools/ws-scrcpy
# The emulator this project starts (serial emulator-5554, as in builder.toml's
# page); `make install` on a USB phone: make install ADB="mise exec -- adb".
ADB = $(MISE) adb -s emulator-5554
AVD = $(HOME)/.android/avd/voipappz.ini
WS_SCRCPY_REF = cd6cea6ebde17af6685c2b325183fc24bef9d52b

.PHONY: setup dev stop test registercheck probe apk install avd emulator launch

# Once: install the tools (mise), the Android SDK packages, Gradle and the
# libraries; compile and run the tests.
setup:
	mise install
	mise run android-sdk
	$(GRADLE) :sip:jvmTest
	$(MAKE) $(AVD) $(WS_SCRCPY)/.built
	touch .builder-setup-done

# While Builder runs: the emulator (started if it is not running), the app
# installed and opened on it, and its screen streamed to Builder's pane.
dev: $(AVD) $(WS_SCRCPY)/.built
	@test "$${PORT:-8000}" = 8000 || { echo "Port 8000 is taken: the emulator stream needs it (builder.toml page)."; exit 1; }
	$(ADB) get-state >/dev/null 2>&1 || $(MAKE) emulator HEADLESS=1
	timeout 300 sh -c 'until [ "$$($(ADB) shell getprop sys.boot_completed 2>/dev/null | tr -d "\r")" = 1 ]; do sleep 2; done' \
	  || { echo "The emulator did not boot in 5 minutes; see build/emulator.log."; exit 1; }
	$(MAKE) install launch ADB="$(ADB)"
	cd $(WS_SCRCPY)/dist && exec $(MISE) node index.js

# After Builder stops `make dev`: shut down this project's emulator only.
stop:
	-$(ADB) emu kill

# The screen stream: ws-scrcpy at a pinned commit with our patch (listen on
# this computer only; accept only Builder's page), built once into .tools/.
# The built server uses the locked dependencies of the checkout.
$(WS_SCRCPY)/.built: tools/ws-scrcpy-local.patch
	rm -rf $(WS_SCRCPY) && mkdir -p $(WS_SCRCPY)
	git -C $(WS_SCRCPY) init -q && git -C $(WS_SCRCPY) fetch -q --depth 1 https://github.com/NetrisTV/ws-scrcpy.git $(WS_SCRCPY_REF)
	git -C $(WS_SCRCPY) checkout -q FETCH_HEAD
	git -C $(WS_SCRCPY) apply $(CURDIR)/tools/ws-scrcpy-local.patch
	cd $(WS_SCRCPY) && npm ci --no-audit --no-fund && npm run dist
	touch $@

test:
	$(GRADLE) :sip:jvmTest

# One live REGISTER against a real switch, then unregister. The account comes
# from the environment only: SIP_USER, SIP_PASSWORD, SIP_WSS (wss://host:port),
# SIP_DOMAIN (optional). The password is never printed.
registercheck:
	$(GRADLE) -q :sip:registerCheck

# The Android app (debug build), and installing it on a phone connected over
# USB with developer mode on (adb from the Android SDK).
apk:
	$(GRADLE) -q :androidApp:assembleDebug
	@echo androidApp/build/outputs/apk/debug/androidApp-debug.apk

install: apk
	$(ADB) install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk

# The Android emulator on this PC (KVM): once `make avd`, then `make emulator`
# (add HEADLESS=1 for no window), then `make install`.
AVD_IMAGE = system-images;android-35;google_apis;x86_64
avd: $(AVD)
$(AVD):
	yes | $(MISE) sdkmanager --install "emulator" "$(AVD_IMAGE)"
	echo no | $(MISE) avdmanager create avd -n voipappz -k "$(AVD_IMAGE)" -d pixel_7 --force

emulator:
	mkdir -p build
	cmd='$(MISE) emulator -avd voipappz -port 5554 -no-audio -no-boot-anim -gpu swiftshader_indirect $(if $(HEADLESS),-no-window)'; \
	  if [ -w /dev/kvm ]; then sh -c "$$cmd"; else sg kvm -c "$$cmd"; fi > build/emulator.log 2>&1 &
	$(ADB) wait-for-device

launch:
	$(ADB) shell monkey -p io.voipappz.phone -c android.intent.category.LAUNCHER 1 >/dev/null

# A connection check with no credentials: one REGISTER, the switch's 401
# challenge printed, nothing answered (no login attempt).
probe:
	SIP_PROBE=1 SIP_WSS=$${SIP_WSS:-wss://switch.voipappz.io:8443} $(GRADLE) -q :sip:registerCheck
