# Mobile phone app: research summary (2026-11-18)

The question: how to build a native mobile phone app (Android first, iPhone
later) that talks to our FreeSWITCH switch the way the browser phone does:
SIP over WebSocket (`wss://switch.voipappz.io:8443`) with WebRTC audio.

The owner's rules: SIP (not Verto), Kotlin (shared code for Android and
iPhone), only permissive licences (MIT, Apache, BSD), a native app (not a web
page in a shell), and no native SIP libraries (no baresip, PJSIP or Linphone),
so it stays easy to develop.

Everything collected: [research/](research/): the full report with sources
([report.md](research/report.md)), the research notes and the three Codex
reviews ([notes/](research/notes/)), and the earlier findings from the working
conversation ([earlier-findings.md](research/earlier-findings.md)).

## The answer

Write our own Kotlin SIP code and borrow everything else. No library with a
permissive licence does SIP over WebSocket in Kotlin and also runs on iPhone.

## What was looked at, and why it was not chosen

| Option | Why not |
|---|---|
| Linphone, PJSIP, Sipdroid, mjSIP, CSipSimple | GPL/AGPL (or paid), and most have no WebSocket |
| baresip | Native C library; ruled out by "no native SIP libraries" |
| JAIN-SIP (Java) | Android only, stale, and its WebSocket client is broken |
| Telnyx Android SDK | Telnyx's own protocol (renamed FreeSWITCH Verto), not SIP |
| FreeSWITCH Verto + a Telnyx fork | Not SIP; needs mod_verto switched on; Android only |
| sip.js inside a JavaScript engine (QuickJS) | Works in theory; no precedent outside React Native; two runtimes to maintain |
| Flutter / React Native | Leaves Kotlin |
| uvp-gb28181-sim (MIT, Kotlin, Android+iPhone) | A camera simulator; useful parts only (see below) |
| webphone-android (MIT, Kotlin, Android) | Closest match, but new and untested against a live switch; a reference |

## Best choice for each layer

| Layer | Best choice | Runner-up |
|---|---|---|
| SIP | Our own Kotlin core, laid out like sip.js; uvp-gb28181-sim's parser after fixes; our own digest code | Fork webphone-android's SIP code (Android only) |
| WebSocket | Ktor (Android and iPhone) | OkHttp (Android only) |
| Call setup (SDP) | Copy sip.js: gather ICE candidates fully with a time limit (no trickle), send unmodified, hold by re-INVITE, DTMF per RFC 4733 | Shorter time limit (1–2 s) |
| Audio, Android | webrtc-kmp (WebRTC M125) | Fork webrtc-kmp onto a newer WebRTC build |
| Audio, iPhone | webrtc-kmp | stasel/WebRTC package |
| System call screen | Core-Telecom (Android), CallKit (iPhone) | Telnyx's call-screen sample |
| Push, phone side | FCM high priority (Android), PushKit (iPhone) | An "always connected" mode for business users |
| Push, switch side | A small Lua flow on the switch: push, wait for the phone to register, connect the call | OpenSIPS in front of FreeSWITCH |
| Network | A STUN server | TURN over UDP, and TLS on port 443 |
| Screens | Compose Multiplatform | Compose (Android) + SwiftUI (iPhone) |
| Testing | Offline tests, then live tests on a real phone | A test FreeSWITCH in CI |

## Key facts found

- FreeSWITCH does not take trickle ICE through mod_sofia, uses UDP candidates
  only, and has no TURN client. sip.js works with it by waiting for ICE
  gathering (5 s limit) and sending the full SDP; we do the same.
- sip.js applies no SDP changes by default. Its last release is 0.21.2 (2022).
- A phone needs about 8–9k lines of sip.js's logic; in Kotlin, roughly 5–7k
  lines, or 2–3k for a first single-call version (estimates).
- webrtc-kmp: Apache-2.0, Android and iPhone, last release v0.125.11 (Sep 2025,
  marked pre-release), WebRTC M125. Fine for audio; fork it if a newer WebRTC is
  needed.
- Incoming calls need push on both phones:
  - Android: a high-priority FCM message starts a `phoneCall` foreground
    service; the call goes to Core-Telecom; a call notification within 5 s.
  - iPhone: every VoIP push must be reported to CallKit at once, or iOS kills
    the app and can stop sending pushes.
  - FreeSWITCH has no built-in support for holding a call until the woken phone
    registers again (RFC 8599); a small Lua flow on the switch does it.
- None of the big calling services (Telnyx, Twilio, Vonage, Sinch) uses SIP
  over WebSocket on phones; the "SIP library + native WebRTC" pattern is proven
  in Flutter and React Native.

## Codex's reviews

- Our SIP core: sound start; fixes applied (digest parsing, bounded retries,
  binary frames, closed connections, 423, password never printed).
- The plan: workable; prove it gate by gate before building screens; the
  biggest gaps are push, TURN, reconnecting and network changes.
- uvp-gb28181-sim: keep our core; copy its parser and some tests; its digest
  code, call cancel handling and TCP keep-alive handling have bugs.

## Proof gates (one attempt each, screens last)

0. Make the SIP module Kotlin Multiplatform; fix the remaining core issues.
1. Live REGISTER, renew, reconnect, unregister against the switch.
2. A call from a real Android phone to an echo extension, two-way audio.
3. Incoming calls, the cancel/answer race, DTMF, hold.
4. Restrictive networks (forced TURN) and Wi-Fi/mobile switching.
5. Push with the switch-side wait flow.
6. iPhone with CallKit and PushKit.

## Not proven yet

- Ktor and webrtc-kmp on iPhone.
- Whether FreeSWITCH keeps push parameters on WebSocket registrations.
- Which TURN server to run.
- The live registration against our switch (gate 1).
