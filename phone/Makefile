# The targets VoipAppz Builder runs (builder.toml). Builder passes PORT to
# every target; the phone app has no web address and ignores it.
#
# Java, the Android SDK and Gradle's own folder: under ~/android unless set.
JAVA_HOME ?= $(HOME)/android/jdk
ANDROID_HOME ?= $(HOME)/android/sdk
GRADLE_USER_HOME ?= $(HOME)/android/gradle
export JAVA_HOME ANDROID_HOME GRADLE_USER_HOME
GRADLE = ./gradlew --console=plain

.PHONY: setup dev test registercheck probe apk install

# Once: download Gradle and the libraries, compile, run the tests.
setup:
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
	$(ANDROID_HOME)/platform-tools/adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk

# A connection check with no credentials: one REGISTER, the switch's 401
# challenge printed, nothing answered (no login attempt).
probe:
	SIP_PROBE=1 SIP_WSS=$${SIP_WSS:-wss://switch.voipappz.io:8443} $(GRADLE) -q :sip:registerCheck
