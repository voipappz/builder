# Earlier findings (2026-11-17 to 2026-11-18)

What was found before the deep research ([report.md](report.md)), collected
from the working conversation so it is not lost. Sources are linked where they
were checked; anything not checked is marked.

## How the direction got here

1. Builder builds React/MUI customer apps, with an Elixir (Connectix) backend: dropped.
2. Builder builds Rust-only native apps from the RustSIPPhone fork, desktop first: superseded.
3. Target embedded: Raspberry Pi first, maybe an ESP32 with Embassy later: superseded.
4. Mobile only, Kotlin Multiplatform, SIP over WebSocket, webrtc-kmp, our own SIP code: **current**.
5. Later: a desktop phone from the RustSIPPhone fork (see [../plan.md](../plan.md)).

Owner's rules: SIP (not Verto), Kotlin, permissive licences only, native (not a
web page), no native SIP libraries (no baresip, PJSIP, Linphone), easy to develop.

## Our switch (checked 2026-11-18)

- Phones connect with SIP over WebSocket to `wss://switch.voipappz.io:8443`,
  the same endpoint the browser phone uses. Its certificate is valid and it
  accepted a WebSocket handshake with subprotocol `sip`; no SIP message has
  been sent to it yet.
- The browser phone keeps the SIP password in browser storage; the mobile app
  must use the platform's secure storage instead (Codex).
- FreeSWITCH's Verto module exists on the switch but is not enabled; the owner
  does not want Verto.

## Projects checked on GitHub

| Project | What | Licence | Verdict |
|---|---|---|---|
| [webrtc-kmp](https://github.com/shepeliev/webrtc-kmp) | WebRTC for Kotlin Multiplatform (Android, iOS, JS); no SIP, no desktop | Apache-2.0 | **Use for audio.** v0.125.11 (Sep 2025, pre-release), WebRTC M125. Its Android sample builds here (29 MB). |
| [telnyx-webrtc-android](https://github.com/team-telnyx/telnyx-webrtc-android) | Telnyx's Android SDK; signalling is FreeSWITCH Verto renamed (`telnyx_rtc.*`) | MIT | Not SIP. Reference for the app layer; its call-screen sample is the separable part. |
| [linphone-android](https://github.com/BelledonneCommunications/linphone-android) | The best-known Android SIP app | GPL-3.0 (SDK AGPL/paid) | Excluded: licence, native library |
| [baresip-studio](https://github.com/juha-h/baresip-studio) | Android SIP app on baresip, supports WSS | BSD-3 | Excluded: native library (owner's rule) |
| [aria-android](https://github.com/5060-Solutions/aria-android) | Compose + ConnectionService, Rust core | none stated | Reference only |
| [sip-calling-android](https://github.com/imavinashex/sip-calling-android) | Kotlin app on Linphone; UDP/TCP/TLS, no WebSocket | README says GPLv3, no licence file | Excluded |
| [IPDial](https://github.com/nazimunaeem/IPDial) | Kotlin/Compose app on PJSIP 2.5 (binary), ConnectionService, ads | no licence file (PJSIP GPL) | Excluded; good `APP_UNDERSTANDING.md` |
| [sipdialer (NoxSIP Lite)](https://github.com/mehedycse/sipdialer) | Kotlin/Compose on Linphone; plugs into the phone's dialer | GPL-3.0 | Excluded; dialer-integration idea worth noting |
| [uvp-gb28181-sim](https://github.com/Frank-Coding97/uvp-gb28181-sim) | GB28181 camera simulator with its own SIP stack in shared Kotlin (Android + iOS) | MIT | Copy the parser and tests; see the Codex review in notes/ |
| [webphone-android](https://github.com/LarsArtmann/webphone-android) | Kotlin, OkHttp SIP over WSS + WebRTC, aimed at FreeSWITCH | MIT | Reference for call flows; untested against a live switch |
| [sip4k](https://github.com/soft-stech/sip4k) | Kotlin SIP/RTP | MIT | Abandoned (2021) |
| [JAIN-SIP](https://github.com/RestComm/jain-sip) | Classic Java SIP stack | public domain (NIST) | JVM only; WebSocket client broken |
| [mjSIP](https://github.com/haumacher/mjSIP) | Java SIP stack | GPL-2.0 | Excluded |
| [react-native-callkeep](https://github.com/react-native-webrtc/react-native-callkeep) | CallKit / ConnectionService for React Native | ISC | Reference for the system call screen |
| [RustSIPPhone](https://github.com/DjTim0n/RustSIPPhone) (fork: [voipappz/RustSIPPhone](https://github.com/voipappz/RustSIPPhone)) | Rust desktop SIP phone (rsipstack, cpal, egui), G.711 only | MIT | Kept for the later desktop phone; builds here |

## Rust on mobile (researched 2026-11-17; superseded)

- egui runs on Android, but its soft keyboard does not work under NativeActivity
  ([egui #5590](https://github.com/emilk/egui/issues/5590), open). iOS works
  unofficially. Slint has official Android and iOS guides; Dioxus mobile is a
  webview by default; Iced does not target mobile.
- cpal 0.18 supports Android (AAudio) and iOS but exposes no platform echo
  cancellation ([cpal PR #995](https://github.com/RustAudio/cpal/pull/995), open);
  pure-Rust AEC exists ([sonora](https://github.com/dignifiedquire/sonora)).
- Ringing when closed needs a small Kotlin/Swift piece whatever the language.
- rsipstack 0.8.6 (2026-11-16) has no SRTP; media via rustrtc.
- Android builds work from Linux/WSL; iOS needs a Mac.

## Embedded (researched 2026-11-17; superseded)

- Raspberry Pi: the desktop Rust phone runs almost as is; the low-risk path.
- Microcontroller: rsipstack 0.8.6 has a `no_std` build with a `platform-embassy`
  backend (compile-checked for ESP32-S3, no demonstrated call). Best board found:
  ESP32-S3-BOX-3 (Wi-Fi, mics, speaker, touch screen; Slint supports it). No
  no_std echo cancellation found; no simulator covers audio.
- A quick egui test drew Hebrew letters correctly but laid text out
  left-to-right; not relevant for an English personal app.

## Business review (Codex, 2026-11-17)

- Pause expanding Builder; it can still pay off as an internal tool if it cuts
  custom-delivery time (test: three real tasks, keep it if it saves 30% or more).
- A Raspberry Pi desk phone is weak as a generic product (cheap Yealink,
  Grandstream, Fanvil phones; hardware, compliance and support costs); it makes
  sense only for a specialised workflow.
- Test demand with existing customers first (five interviews, a mock-up, two
  paid pilots). A mobile softphone for existing customers is a candidate.

## Our code (state on 2026-11-18)

- `phone/sip`: our SIP-over-WebSocket code in common Kotlin (messages, digest,
  registration, client, WebSocket transport); 17 tests; Codex-reviewed and fixed.
- `phone/shared`, `phone/androidApp`: JetBrains' Kotlin Multiplatform layout;
  the Android app builds (11 MB) with a sign-in screen.
- Builder runs the phone app (`phone/builder.toml`, `kind = "app"`).
- Everything is on branch `feat/phone-app` in `~/voipappz/builder-native`, not pushed.
