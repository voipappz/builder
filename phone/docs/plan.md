# VoipAppz Phone: plan

Research behind it: [research-summary.md](research-summary.md); everything
collected in [research/](research/). Where everything is and what is next:
[state.md](state.md).

The app's design (screens, look, flows) is open for discussion; the gates
below are about making calls work first.

## Now: the mobile app (Kotlin Multiplatform)

Android first, iPhone after. SIP over WebSocket to `wss://switch.voipappz.io:8443`
with our own Kotlin SIP code, webrtc-kmp for audio, Compose Multiplatform
screens. One gate at a time, screens last.

| Gate | What | State |
|---|---|---|
| 0 | SIP module is Kotlin Multiplatform; JetBrains' layout (`sip`, `shared`, `androidApp`) | Android and desktop-JVM build and test. iOS targets declared, never built (needs a Mac) |
| 1a | **Demo 1:** a real phone registers (REGISTER, 401, digest, 200) and unregisters | Code done. `make probe` reached the switch's 401 challenge (MD5, qop=auth) on 2026-11-18; a login with a real extension is pending |
| 1b | Staying registered: renewal, reconnect, a dispatcher for requests the switch sends | Not built |
| 2 | **Demo 2:** a call to an echo extension with audible echo both ways (webrtc-kmp), media statistics, a second call that cleans up after the first, microphone permission | Next |
| 3 | iOS on a Mac (framework links, webrtc-kmp audio, one call), and push/Core-Telecom/CallKit feasibility | Arrange Mac and iPhone access now |
| 4 | Incoming calls, the cancel/answer race, DTMF, hold; audio routing and interruptions | |
| 5 | Secure storage of the SIP password (Keystore, Keychain), before push (a woken app needs the credentials) | |
| 6 | Push for incoming calls: FCM, PushKit + CallKit, a Lua flow on the switch; late, duplicate and cancelled pushes | The switch-side flow is a design, untested |
| 7 | Restrictive networks (TURN) and Wi-Fi/mobile switching, before any pilot | |
| 8 | The real screens: dial pad, recent calls, contacts, settings | Design open for discussion |

## Later: a desktop phone

When the mobile app is done, a desktop phone (Windows, macOS, Linux) based on
our Rust fork of RustSIPPhone:

- Fork: https://github.com/voipappz/RustSIPPhone (MIT; rsipstack, cpal, egui).
- Local work kept for it: branch `builder-files` in `~/voipappz/phone-template`
  (Builder files, device screen size); evaluation copy in
  `~/voipappz/eval/RustSIPPhone`.
- Builder already runs it as a native app (`kind = "app"`).
- **Prerequisite before any desktop work (Codex):** the Rust phone sends plain
  RTP (G.711, RTP/AVP), with no WebRTC, DTLS-SRTP or ICE. Over the switch's WSS
  WebRTC endpoint it could register yet have no audio. Prove first that the
  switch has a reachable SIP profile accepting plain RTP, or budget replacing
  its media layer.
- **Compare before committing:** Compose Multiplatform desktop (JVM) with
  [webrtc-java](https://github.com/devopvoid/webrtc-java) (Apache-2.0, Windows,
  macOS, Linux) would reuse the Kotlin SIP code, call logic and much of the UI;
  the Rust fork reuses a finished desktop phone but means a second codebase.
- If two codebases: one written spec of the SIP behaviour, shared test vectors
  (digest, SDP, call states), one feature list, and the same interop tests
  against the switch for both.
