# VoipAppz Phone

A mobile phone app (Android first, iPhone later) in Kotlin Multiplatform. It
talks to the VoipAppz switch the way the browser phone does: SIP over
WebSocket to `wss://switch.voipappz.io:8443`, with WebRTC audio (webrtc-kmp,
not wired in yet). Plan: [docs/plan.md](docs/plan.md). Research:
[docs/research-summary.md](docs/research-summary.md).

## Layout (JetBrains' Kotlin Multiplatform template)

| Module | What |
|---|---|
| `sip/` | Our SIP-over-WebSocket code, common Kotlin. Tests in `commonTest` (all platforms) and `jvmTest` (desktop) |
| `shared/` | The app: Compose Multiplatform screens and logic, for Android and iPhone |
| `androidApp/` | The Android entry point |
| `iosApp/` | The iPhone entry point (Xcode project). Not added yet: needs a Mac |

## What you need

- [mise](https://mise.jdx.dev). `make setup` (or `mise install` then
  `mise run android-sdk`) installs Java 17 (Temurin) and the Android SDK with
  platform 35, build tools and platform-tools, as listed in `mise.toml`. Every
  `make` command runs through `mise exec`, so mise does not need to be
  activated in your shell.
- For iPhone: a Mac with Xcode.

## Commands

| Command | What it does |
|---|---|
| `make setup` | First run: installs the tools with mise, then Gradle and the libraries; runs the tests |
| `make test` | The SIP tests (desktop JVM) |
| `make dev` | Re-runs the tests on every change (what VoipAppz Builder runs) |
| `make apk` | Builds the Android app: `androidApp/build/outputs/apk/debug/androidApp-debug.apk` |
| `make install` | Installs it on an Android phone connected over USB (developer options and USB debugging on; accept the computer's prompt on the phone). Then open "VoipAppz Phone" |
| `make probe` | Connection check with no credentials: the switch's 401 challenge, nothing answered |
| `make registercheck` | One live REGISTER against the switch, then unregister (see below) |

## SIP accounts and secrets

Nothing secret is in this repository. An extension and its password come from
the VoipAppz admin (the account's Devices); you type them yourself:

- `make registercheck` reads `SIP_USER`, `SIP_PASSWORD`, `SIP_WSS`
  (e.g. `wss://switch.voipappz.io:8443`) and, optionally, `SIP_DOMAIN` (default:
  the WSS host) from the environment. It never prints the password. To keep the
  password out of your shell history:

  ```sh
  read -rp "Extension: " SIP_USER && read -rsp "Password: " SIP_PASSWORD && echo
  SIP_USER=$SIP_USER SIP_PASSWORD=$SIP_PASSWORD SIP_WSS=wss://switch.voipappz.io:8443 make -s registercheck
  ```

- The app's test screen keeps the password in memory only. Secure storage
  (Android Keystore, iOS Keychain) is a later gate.

## In VoipAppz Builder

`builder.toml` lets Builder work on this app: Builder runs `make dev`, its chat's
coding agent edits the code, and Status has buttons for tests, building the
Android app and installing it on a phone.
