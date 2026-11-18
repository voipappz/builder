# The targets VoipAppz Builder runs (builder.toml). Builder shows the Android
# emulator's screen, streamed by ws-scrcpy (MIT) on port 8000, in its pane
# beside the chat. Builder passes PORT to every target; it is not used.
#
# Java 17 and the Android SDK come from mise (mise.toml): every command runs
# through `mise exec`, so it works with or without mise activated in the shell.
MISE = mise exec --
GRADLE = $(MISE) ./gradlew --console=plain
WS_SCRCPY = .tools/ws-scrcpy
WS_SCRCPY_REF = cd6cea6ebde17af6685c2b325183fc24bef9d52b

.PHONY: setup dev stop test registercheck probe apk install avd emulator launch

# Once: install the tools (mise), the Android SDK packages, Gradle and the
# libraries; compile and run the tests.
setup:
	mise install
	mise run android-sdk
	$(GRADLE) :sip:jvmTest
	test -f $$HOME/.android/avd/voipappz.ini || $(MAKE) avd
	$(MAKE) $(WS_SCRCPY)/dist/index.js
	touch .builder-setup-done

# While Builder runs: the emulator (started if none is running), the app
# installed and opened on it, and its screen streamed to Builder's pane.
dev: $(WS_SCRCPY)/dist/index.js
	$(MISE) adb get-state >/dev/null 2>&1 || $(MAKE) emulator HEADLESS=1
	until [ "$$($(MISE) adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; do sleep 2; done
	$(MAKE) install launch
	cd $(WS_SCRCPY)/dist && exec $(MISE) node index.js

# After Builder stops `make dev`: shut the emulator down.
stop:
	-$(MISE) adb emu kill

# The screen stream: ws-scrcpy at a pinned commit, built once into .tools/.
$(WS_SCRCPY)/dist/index.js:
	rm -rf $(WS_SCRCPY) && mkdir -p $(WS_SCRCPY)
	git -C $(WS_SCRCPY) init -q && git -C $(WS_SCRCPY) fetch -q --depth 1 https://github.com/NetrisTV/ws-scrcpy.git $(WS_SCRCPY_REF)
	git -C $(WS_SCRCPY) checkout -q FETCH_HEAD
	# Listen on this computer only: the page controls the emulator (and has a shell).
	sed -i "s/server.listen(port, () =>/server.listen(port, '127.0.0.1', () =>/" $(WS_SCRCPY)/src/server/services/HttpServer.ts
	grep -q "server.listen(port, '127.0.0.1'" $(WS_SCRCPY)/src/server/services/HttpServer.ts
	cd $(WS_SCRCPY) && npm ci --no-audit --no-fund && npm run dist
	cd $(WS_SCRCPY)/dist && npm install --omit=dev --no-audit --no-fund

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
	$(MISE) adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk

# The Android emulator on this PC (KVM): once `make avd`, then `make emulator`
# (add HEADLESS=1 for no window), then `make install`.
AVD_IMAGE = system-images;android-35;google_apis;x86_64
avd:
	yes | $(MISE) sdkmanager --install "emulator" "$(AVD_IMAGE)"
	echo no | $(MISE) avdmanager create avd -n voipappz -k "$(AVD_IMAGE)" -d pixel_7 --force

emulator:
	mkdir -p build
	sg kvm -c '$(MISE) emulator -avd voipappz -no-audio -no-boot-anim -gpu swiftshader_indirect $(if $(HEADLESS),-no-window)' > build/emulator.log 2>&1 &
	$(MISE) adb wait-for-device

launch:
	$(MISE) adb shell monkey -p io.voipappz.phone -c android.intent.category.LAUNCHER 1 >/dev/null

# A connection check with no credentials: one REGISTER, the switch's 401
# challenge printed, nothing answered (no login attempt).
probe:
	SIP_PROBE=1 SIP_WSS=$${SIP_WSS:-wss://switch.voipappz.io:8443} $(GRADLE) -q :sip:registerCheck
