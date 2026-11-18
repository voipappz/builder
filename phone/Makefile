# The targets VoipAppz Builder runs (builder.toml). Builder passes PORT to
# every target; the phone app has no web address and ignores it.
#
# Java 17 and the Android SDK come from mise (mise.toml): every command runs
# through `mise exec`, so it works with or without mise activated in the shell.
MISE = mise exec --
GRADLE = $(MISE) ./gradlew --console=plain

.PHONY: setup dev test registercheck probe apk install avd emulator

# Once: install the tools (mise), the Android SDK packages, Gradle and the
# libraries; compile and run the tests.
setup:
	mise install
	mise run android-sdk
	$(GRADLE) :sip:jvmTest
	touch .builder-setup-done

# While Builder runs: re-run the SIP tests on every change.
dev:
	$(GRADLE) --continuous :sip:jvmTest

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
	sg kvm -c '$(MISE) emulator -avd voipappz -no-audio -no-boot-anim -gpu swiftshader_indirect $(if $(HEADLESS),-no-window)' &
	$(MISE) adb wait-for-device

# A connection check with no credentials: one REGISTER, the switch's 401
# challenge printed, nothing answered (no login attempt).
probe:
	SIP_PROBE=1 SIP_WSS=$${SIP_WSS:-wss://switch.voipappz.io:8443} $(GRADLE) -q :sip:registerCheck
