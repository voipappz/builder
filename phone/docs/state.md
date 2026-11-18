# State, 2026-11-18 (to pick up next week)

## Where things are

| What | Where | State |
|---|---|---|
| Phone app (Kotlin Multiplatform) | `phone/` in voipappz/builder, branch `feat/phone-app` | SIP registration code, Android test app, docs; pushed with this branch |
| Plan | [plan.md](plan.md) | Gates 0–8 for mobile; desktop (Rust) later |
| Research | [research-summary.md](research-summary.md), [research/](research/) | Full report, notes, six Codex reviews, earlier findings |
| Builder: native-app support (`kind = "app"`) | voipappz/builder, same branch | Done, tested |
| Builder: chat from connectix-io/app | voipappz/builder branch `keep/chat-from-connectix-io` (local) | Waits for connectix-io/app `feat/builder-chat` to merge |
| Builder's chat copied into the template app | `~/voipappz/connectix-io-app`, branch `feat/builder-chat` (local) | Not pushed; pushing makes the chat code public |
| Rust desktop phone (later) | fork `voipappz/RustSIPPhone`; local branch `builder-files` in `~/voipappz/phone-template`; copy in `~/voipappz/eval/RustSIPPhone` | Kept untouched on purpose |
| References | `~/voipappz/eval/uvp-gb28181-sim` (parser to copy), `~/voipappz/eval/telnyx-webrtc-android` (Android call screen) | Local clones |
| Tools | `phone/mise.toml`: Java 17 (Temurin), Android SDK | `make setup` installs them |
| CI | `.github/workflows/phone.yml`: SIP tests and the Android app on every change to `phone/` | Runs on GitHub |

## What was proven

- The switch answers our SIP over WebSocket: `make probe` on 2026-11-18 got
  `401 Unauthorized`, realm `switch.voipappz.io`, MD5, `qop=auth` (what our digest
  supports). No login with a real extension yet.
- 17 SIP tests pass; the Android app builds (debug, about 11 MB); the shared
  code compiles for a native target (iPhone not built: needs a Mac).

## Next week, in order

1. **Demo 1 (gate 1a):** log in with a real extension. Either install the app
   (`VoipAppz-Phone-demo-0.1.0.apk` was put in Windows Downloads; CI also
   uploads it) and press Register, or run `make registercheck` with the
   extension in the environment (see [../README.md](../README.md)).
2. **Gate 1b:** renewal, reconnect, and a dispatcher for requests the switch
   sends (OPTIONS keep-alive, incoming INVITE).
3. **Demo 2 (gate 2):** a call to an echo extension with webrtc-kmp audio.
   Needs: an echo extension on the switch; a test extension whose password is
   saved by the owner in `~/.config/voipappz-phone/test.env` (never read into
   chat); and an Android phone over USB, or the emulator after
   `sudo usermod -aG kvm $USER` and a WSL restart.
4. Arrange Mac and iPhone access for gate 3.

## Open decisions for the owner

- The app's design (screens, look, flows): open for discussion.
- Desktop phone: the Rust fork vs a Compose desktop app with webrtc-java (see
  the plan's desktop section; the Rust fork has no WebRTC audio).
- Pushing connectix-io/app `feat/builder-chat` (makes Builder's chat code public).
