# VoipAppz Phone: plan

Research behind it: [research-summary.md](research-summary.md); everything
collected in [research/](research/).

The app's design (screens, look, flows) is open for discussion; the gates
below are about making calls work first.

## Now: the mobile app (Kotlin Multiplatform)

Android first, iPhone after. SIP over WebSocket to `wss://switch.voipappz.io:8443`
with our own Kotlin SIP code, webrtc-kmp for audio, Compose Multiplatform
screens. One gate at a time, screens last.

| Gate | What | State |
|---|---|---|
| 0 | SIP module is Kotlin Multiplatform; JetBrains' layout (`sip`, `shared`, `androidApp`) | Android and desktop-JVM build and test. iOS targets are declared but never built (needs a Mac) |
| 1 | Live REGISTER and unregister against the switch; then renewal and reconnect | REGISTER/unregister code done, live test pending (`make registercheck`, or the app's screen). Renewal and reconnect not built |
| 2 | A call from a real Android phone to an echo extension, two-way audio (webrtc-kmp), with the microphone permission | Next |
| 3 | iOS feasibility on a Mac: the framework links, webrtc-kmp plays audio, one call | Early, to catch iOS problems before the design is built on top |
| 4 | Incoming calls, the cancel/answer race, DTMF, hold; audio routing (earpiece, speaker, Bluetooth) and interruptions (a mobile call arriving) | |
| 5 | Restrictive networks (TURN) and Wi-Fi/mobile switching | |
| 6 | Push for incoming calls: FCM (Android), PushKit + CallKit (iPhone), a Lua flow on the switch; tests for late, duplicate and cancelled pushes | The switch-side flow is a design, untested |
| 7 | Secure storage of the SIP password (Android Keystore, iOS Keychain) | |
| 8 | The real screens: dial pad, recent calls, contacts, settings | Design open for discussion |

## Later: a desktop phone

When the mobile app is done, a desktop phone (Windows, macOS, Linux) based on
our Rust fork of RustSIPPhone:

- Fork: https://github.com/voipappz/RustSIPPhone (MIT; rsipstack, cpal, egui).
- Local work kept for it: branch `builder-files` in `~/voipappz/phone-template`
  (Builder files, device screen size); evaluation copy in
  `~/voipappz/eval/RustSIPPhone`.
- Builder already runs it as a native app (`kind = "app"`).
- To decide then: share behaviour with the mobile app (same switch settings,
  same features) while the code stays separate (Kotlin on phones, Rust on
  desktop).
