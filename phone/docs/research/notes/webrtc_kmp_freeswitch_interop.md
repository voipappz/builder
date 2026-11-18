# WebRTC for KMP mobile softphones and native-libwebrtc interop with FreeSWITCH SIP-over-WSS (as of 2026-11-18)

Note on dates: GitHub release pages fetched for this research show day and month but not the year. Years given below are inferred from the order of releases and from the dates of linked issues. Mailing-list findings carry their posting dates.

## 1. webrtc-kmp: state, maintenance and issues, plus alternative WebRTC builds for Android and iOS

### Takeaway
webrtc-kmp (shepeliev) is still maintained, though lightly. Its latest release is v0.125.11 (superseded: the GitHub API dates it 2025-09-07, not Sep 2026 as first guessed here). It is pinned to WebRTC **M125** (mid-2024), which is roughly 30 milestones behind current upstream builds. Its open issues are mostly about video and the build, not audio. It remains the only real KMP wrapper, and it is usable for audio-only calls. For newer WebRTC there are fresher binary builds: webrtc-sdk/LiveKit for Android (M137, possibly M144) and stasel/WebRTC for iOS (up to M157 on its page). Using them means writing your own thin expect/actual layer or forking webrtc-kmp to bump its dependencies.

### Cited Findings
- **Latest releases (webrtc-kmp):** v0.125.11 (07 Sep) fixes Android rootEglBase init. v0.125.10 (11 Aug) adds iOS getStats and renames AudioTrack/VideoTrack. v0.125.9 (14 Feb) adds native audio volume control on iOS and Android. v0.125.8 (24 Jan) moves to Kotlin 2.1.0, adds the continual gathering policy, and changes Android RtpSender track ownership (breaking). All are marked "Pre-release". The page shows no year; from the sequence these are most likely 2025 to 2026, with v0.125.11 = Sep 2026. — [webrtc-kmp releases](https://github.com/shepeliev/webrtc-kmp/releases)
- **Native WebRTC versions (main branch):** Android `io.github.webrtc-sdk:android:125.6422.06.1`, iOS WebRTC-SDK `125.6422.07`. Also Kotlin 2.1.21, minSdk 21, compileSdk 35, Compose MP 1.7.3. So it is **M125**. — [webrtc-kmp libs.versions.toml](https://raw.githubusercontent.com/shepeliev/webrtc-kmp/main/gradle/libs.versions.toml)
- **Open issues: 14 in total.** Audio: only #104 "Receiving audio samples" (Mar 2024). iOS: #192, a use-after-scope in `CValue.copyContents()` that garbles camera formats and freezes video (Jul 7, 2026); #122 podBuildWebRTC-SDKIphoneos (Jul 2024); #102 iosMain error (Jan 2024); #171 dSYM for WebRTC.framework (Apr 2025). Other issues: SurfaceView rendering on Android (#38, 2022), wasmJs DataChannel data loss (#189, Jan 2026), "can't start at the first time" (#183, Jul 2025), plus feature requests for desktop, screen capture and rnnoise. — [webrtc-kmp issues](https://github.com/shepeliev/webrtc-kmp/issues)
- **Platforms:** releases mention Android and iOS. Issues also show JS/wasmJs targets (#189, #94). There is **no desktop/JVM target** (#51 "WebRTC on Desktop" is still open). — [webrtc-kmp issues](https://github.com/shepeliev/webrtc-kmp/issues)
- **webrtc-sdk/android (the LiveKit-maintained libwebrtc build used by webrtc-kmp):** the README lists `io.github.webrtc-sdk:android:137.7151.05`, and a `android-prefixed` variant also exists. Maven Central showed 137.7151.04. A mirror shows `144.7559.01` without a date (unverified). — [webrtc-sdk/android](https://github.com/webrtc-sdk/android); [Maven Central](https://central.sonatype.com/artifact/io.github.webrtc-sdk/android)
- **GetStream stream-webrtc-android:** latest release 1.3.10 (11 Sep), which exposes audio attributes. The last release that names a milestone is 1.3.5 = **M125.4**. So it is also still on M125-era WebRTC. — [GetStream/webrtc-android releases](https://github.com/GetStream/webrtc-android/releases)
- **stasel/WebRTC (iOS/macOS binary XCFramework):** frequent releases. M157 (05 Nov, branch-head 8103), M156 (27 Oct), M155 (12 Oct), M154 (30 Sep), M153 (11 Sep), and so on. The milestone numbers look high for Nov 2026 and should be checked against Chromium's schedule, but the page lists them. — [stasel/WebRTC releases](https://github.com/stasel/WebRTC/releases)
- **Telnyx WebRTC Android SDK:** 3.8.0 (release note dated 3 Nov 2026) adds ICE restart, call diagnostics and a "newer bundled WebRTC engine". It is distributed through JitPack (team-telnyx/telnyx-webrtc-android), and it exposes a trickle-ICE toggle (`TelnyxClient.setUseTrickleIce()`). It is coupled to Telnyx's own signalling, not generic SIP. — [Telnyx release note](https://telnyx.com/release-notes/webrtc-android-ice-restart); [Telnyx changelog](https://developers.telnyx.com/docs/development/webrtc/android-sdk/changelog)

### Inferences
- For an **audio-only** SIP softphone, M125 is functionally enough. Opus, G.711, DTLS-SRTP, rtcp-mux and BUNDLE have all been stable for years. The main costs of the old build are missing security patches and older Android/iOS audio-device fixes. Most open webrtc-kmp bugs are in video or the build, so they barely affect audio-only use.
- **Recommended path:** start with webrtc-kmp, which has the fastest KMP route and the same artifact family as LiveKit's builds. If you need newer WebRTC, fork it and bump `io.github.webrtc-sdk:android` to 137.x and the iOS WebRTC-SDK pod (or stasel's XCFramework). The alternative is a ~500-line expect/actual wrapper covering only PeerConnection, audio tracks and SDP, which is all a SIP phone needs.
- **Licences (from general knowledge, not checked this session):** libwebrtc is BSD-3-Clause. webrtc-kmp is Apache-2.0. stream-webrtc-android is Apache-2.0 (patches) with BSD libwebrtc. stasel/WebRTC redistributes BSD-licensed WebRTC binaries. All are compatible with closed-source apps when attribution is kept.

### Gaps
- I did not confirm exact release years on the GitHub pages; the year display was missing.
- I did not get a version or date for the iOS WebRTC-SDK pod (webrtc-sdk/Specs) newer than 125.6422.07.
- I did not verify the licence texts this session.
- I did not verify whether `io.github.webrtc-sdk:android:144.x` is official, or its date.
- I found no other maintained KMP WebRTC wrapper. I did not search exhaustively; candidates would be forks of webrtc-kmp.

## 2. What sip.js does to SDP, and what FreeSWITCH WSS endpoints require

### Takeaway
sip.js applies **no SDP modifiers by default**. It does not trickle: it waits for ICE gathering to complete, with a default 5000 ms cap in the default SDH factory, and then sends one full offer or answer containing all candidates. It implements hold by changing the offer direction (sendrecv→sendonly) and re-INVITEing. On the FreeSWITCH side, DTLS-SRTP is activated by `a=fingerprint`. FreeSWITCH is a full ICE agent by default (ICE-lite only via the `ice_lite` channel variable), handles UDP candidates only, filters candidates through `apply-candidate-acl`, never generates relay candidates and has no TURN client. A native phone that mimics sip.js gets the same behaviour: non-trickle full gathering with a timeout, unmodified Unified-Plan SDP, and hold through direction changes.

### Cited Findings
**sip.js (main branch source):**
- `SessionDescriptionHandler.getDescription()` calls `waitForIceGatheringComplete(iceRestart, iceTimeout)` and then returns the local description. `onicecandidate` only forwards events to an optional delegate, so there is **no trickle**. `iceGatheringComplete()` lets the app release early with the candidates gathered so far. — [session-description-handler.ts](https://raw.githubusercontent.com/onsip/SIP.js/main/src/platform/web/session-description-handler/session-description-handler.ts)
- The default factory sets `iceGatheringTimeout` to **5000 ms** ("The ICE gathering timeout defaults to 5000ms"); `0` disables the timeout. — [session-description-handler-factory-default.ts](https://raw.githubusercontent.com/onsip/SIP.js/main/src/platform/web/session-description-handler/session-description-handler-factory-default.ts)
- `applyModifiers` returns the SDP unchanged when no modifiers are passed. There are no default SDP modifiers. — [session-description-handler.ts](https://raw.githubusercontent.com/onsip/SIP.js/main/src/platform/web/session-description-handler/session-description-handler.ts)
- **Hold, in `updateDirection`:** when offering with hold, sendrecv→sendonly and recvonly→inactive. When offering without hold, inactive/recvonly→recvonly and sendonly/sendrecv→sendrecv. When answering, the direction is derived from the remote offer by regex (sendrecv→sendonly if on hold, otherwise sendrecv; sendonly→inactive if on hold, otherwise recvonly; recvonly→sendonly; inactive→inactive). The source carries a FIXME saying this is not fully correct. — [session-description-handler.ts](https://raw.githubusercontent.com/onsip/SIP.js/main/src/platform/web/session-description-handler/session-description-handler.ts)
- **Optional exported modifiers:** `stripTcpCandidates`, `stripTelephoneEvent`, `stripG722`, `stripRtpPayload(pt)`, `stripVideo` (also fixes the group line), `addMidLines`, `holdModifier` (direction change for hold) and `cleanJitsiSdpImageattr`. None mention FreeSWITCH. — [modifiers.ts](https://raw.githubusercontent.com/onsip/SIP.js/main/src/platform/web/modifiers/modifiers.ts)
- **The ceiling is a real wait:** reports say SIP.js can hold back the INVITE/200 for the whole timeout in some cases, even when gathering finished quickly (often because a srflx/relay host never answers). — [sip_js group "Big pause while call establishing"](https://groups.google.com/g/sip_js/c/Lt6KhYqDdUc); [freeswitch-users 2022-09, iceCheckingTimeout and sip.js](https://lists.freeswitch.org/pipermail/freeswitch-users/2022-September/135915.html)

**FreeSWITCH WSS/WebRTC (SignalWire developer docs, "Chapter 10: WebRTC over SIP (WSS)". This is a newer docs page; I could not verify each claim against source.)**
- **Listeners:** `ws-binding` (e.g. `:5066`, for dev) and `wss-binding` (e.g. `:7443`, for production) on a Sofia profile. The WSS cert lookup order is `wss.key`+`wss.crt`, then `wss.pem`, then an auto-generated self-signed cert. DTLS uses a separate `dtls-srtp.pem`, also auto-generated. — [FS docs: WebRTC over SIP](https://developer.signalwire.com/freeswitch/users-and-endpoints/webrtc-sip/)
- **DTLS-SRTP** is mandatory and has no profile parameter. It is activated automatically when the offer has `a=fingerprint`. SHA-256 is the default, and FreeSWITCH mirrors the remote hash type. **a=setup:** FreeSWITCH offers `actpass`. If the remote sends `actpass` or `passive`, FreeSWITCH answers `active`; if the remote sends `active`, it answers `passive`. `rtp_secure_media` covers SDES only. — [FS docs](https://developer.signalwire.com/freeswitch/users-and-endpoints/webrtc-sip/)
- **ICE:** full ICE agent by default. FreeSWITCH advertises a `host` candidate from `rtp-ip`, plus a `srflx` candidate from `ext-rtp-ip` when it differs. It generates **no relay candidates** but accepts relay candidates from the client. **Only UDP candidates** are processed. ICE-lite is used only when the channel variable `ice_lite=true` is set. — [FS docs](https://developer.signalwire.com/freeswitch/users-and-endpoints/webrtc-sip/)
- **`apply-candidate-acl`** is repeatable, and non-matching candidates are silently dropped. When unset it defaults to `wan.auto` with a warning. If every non-relay candidate is dropped, FreeSWITCH falls back to the client's relay candidates. — [FS docs](https://developer.signalwire.com/freeswitch/users-and-endpoints/webrtc-sip/)
- **`ext-rtp-ip`** accepts a literal IP, `stun:host[:port]` (resolved once at profile load), `host:name`, `auto` or `auto-nat`. It is also used as the SDP `c=` address. STUN runs only at load. **FreeSWITCH has no TURN client.** — [FS docs](https://developer.signalwire.com/freeswitch/users-and-endpoints/webrtc-sip/)
- **No trickle ICE via mod_sofia:** a FreeSWITCH maintainer answered "No" to whether full trickle works via mod_sofia (Dec 2020). — [freeswitch-users 2020-12](https://lists.freeswitch.org/pipermail/freeswitch-users/2020-December/134315.html); see also [2020-11 thread](https://lists.freeswitch.org/pipermail/freeswitch-users/2020-November/134306.html)
- **Candidate handling quirk (Aug 2021):** a user reported FreeSWITCH sending STUN checks only to the **first** candidate in the list, under both mod_sofia and mod_verto (shared media stack). No reply was archived. — [freeswitch-users 2021-08](https://lists.freeswitch.org/pipermail/freeswitch-users/2021-August/134932.html)
- **Wrong candidate chosen with multi-interface clients (2016).** — [freeswitch-users 2016-09](https://lists.freeswitch.org/pipermail/freeswitch-users/2016-September/122845.html)
- **NAT advice (legacy confluence):** set `ext-sip-ip`/`ext-rtp-ip` to the public IP, and use `local-network-acl` with `rfc1918.auto` and the `autonat:` prefix. — [confluence WebRTC](https://confluence.freeswitch.org/display/FREESWITCH/WebRTC)

### Inferences
- **Recipe a native client should follow to match sip.js:**
  - Gather fully, with no trickle.
  - Cap the gathering wait at 1–5 s and also release early once a usable srflx (or relay) candidate exists.
  - Send the full `localDescription` (not the createOffer output) in the INVITE or 200.
  - Leave the SDP unmodified, apart from optionally stripping TCP candidates, since FreeSWITCH ignores them anyway.
  - Use `rtcpMuxPolicy=require`, `bundlePolicy=balanced` or `max-bundle`, Unified Plan, and an audio-only transceiver.
  - Do hold as a re-INVITE with sendonly, and resume with sendrecv.
  - Answer FreeSWITCH re-INVITEs through setRemoteDescription and createAnswer on the same PeerConnection.
- **Unverified from memory:** sip.js's default `peerConnectionConfiguration` is `bundlePolicy: "balanced"`, `rtcpMuxPolicy: "require"`, and `iceServers: [{urls: "stun:stun.l.google.com:19302"}]`. The file was not fetched, so treat this as unverified.
- **Server side:** set `apply-candidate-acl` so that mobile CGNAT and public candidates are accepted (the `wan.auto` default usually does this). Make sure `ext-rtp-ip` is the server's public IP. Make sure the codec string includes OPUS and PCMU/PCMA.
- Because FreeSWITCH checks candidates in a limited way (first candidate only, per the 2021 report), putting a reachable candidate first is a reasonable mitigation, i.e. not leading with an unreachable private host candidate. This is inference only.
- sip.js v0.21.x is the current line. Development has been quiet since about 2022–2023 (from memory, not re-verified).

### Gaps
- I did not verify `media_webrtc=true` in the official docs this session. From general knowledge it is a channel variable used when FreeSWITCH *originates* a call to a WebRTC endpoint, so the outbound SDP uses SAVPF, ICE and DTLS. Dialplans normally set it automatically for WS/WSS-registered users.
- The FS docs page fetched says nothing about BUNDLE, rtcp-mux, ptime or re-INVITE specifics. From general knowledge FreeSWITCH supports rtcp-mux and audio-only BUNDLE, but this was not verified.
- I found no current (2024–2026) FreeSWITCH GitHub issues describing trickle support. The latest evidence is that trickle is unsupported (2020–2021).

## 3. Known interop problems between native libwebrtc and FreeSWITCH; how dart-sip-ua and flutter-webrtc handle it

### Takeaway
There are few documented native-specific incompatibilities. Most reported failures are generic: missing DTLS fingerprint, codec mismatch (488), NAT/candidate ACLs, and DTLS handshakes that never complete. dart-sip-ua (a JsSIP port on flutter-webrtc/libwebrtc) is the closest analogue to a KMP stack. It also uses non-trickle gathering with a short default timeout (500 ms) and needs DTLS/SRTP from the PBX.

### Cited Findings
- **dart-sip-ua README:** FreeSWITCH, Asterisk, Kamailio, OpenSIPS and 3CX are listed as supported. "Your PBX must be configured to use DTLS/SRTP when calling sip_ua". `WEBRTC_SET_REMOTE_DESCRIPTION_ERROR` occurs when the server's SDP has no DTLS fingerprint. A 488 means a codec mismatch; the expected codecs are Opus, G.722, PCMU, PCMA and telephone-event. DTMF goes via RFC 2833 or SIP INFO. The repo shows about 163 open issues. — [dart-sip-ua](https://github.com/flutter-webrtc/dart-sip-ua)
- **dart-sip-ua `ice_gathering_timeout`** defaults to **500 ms** according to the package docs, as reported by search. The flutter_pitel_voip fork also uses 500 ms. — [pub.dev sip_ua](https://pub.dev/documentation/sip_ua/latest/); [flutter_pitel_voip iceGatheringTimeout](https://pub.dev/documentation/flutter_pitel_voip/latest/sip_src_sip_ua_helper/PitelSettings/iceGatheringTimeout.html)
- **Native Android libwebrtc ↔ FreeSWITCH (2015):** `setRemoteDescription` failed with "Called with SDP without DTLS fingerprint" on an offer from FreeSWITCH. The maintainer's answer was that FreeSWITCH must be told to send WebRTC-compatible SDP (i.e. `media_webrtc` or a WS/WSS-registered user). — [freeswitch-users 2015-03](https://lists.freeswitch.org/pipermail/freeswitch-users/2015-March/111726.html)
- **`a=extmap-allow-mixed`** has been offered by default since Chrome/libwebrtc M89. Very old stacks choke on it, and libwebrtc has an `offerExtmapAllowMixed:false` option. I found no confirmed FreeSWITCH failure caused by it. — [discuss-webrtc PSA](https://groups.google.com/g/discuss-webrtc/c/24LiX06HwpM); [FS issue #1147](https://github.com/signalwire/freeswitch/issues/1147)
- **FreeSWITCH 1.10 + JsSIP:** calls failed with INCOMPATIBLE_DESTINATION and a DTLS handshake that starts but never completes. No resolution was in the snippet. — [FS issue #2976 (mirror)](https://github.huabts.workers.dev/signalwire/freeswitch/issues/2976)
- **Older DTLS delay:** Firefox's 5 s gathering timeout, and extra time when many srflx candidates must be tried (2018). — [freeswitch-users 2018-02](https://lists.freeswitch.org/pipermail/freeswitch-users/2018-February/129048.html)

### Inferences
- Native libwebrtc and browser libwebrtc produce essentially the same SDP, so whatever works with sip.js in Chrome will work from webrtc-kmp, provided the client sends the same SDP: full candidates, Unified Plan, audio only.
- The native-specific risks are app-level rather than SDP-level:
  - Audio session/routing: iOS AVAudioSession and CallKit, Android AudioManager MODE_IN_COMMUNICATION.
  - The app being backgrounded mid-call.
  - Network changes (Wi-Fi↔LTE), which need an ICE restart through a re-INVITE. Whether FreeSWITCH accepts re-INVITE ICE restarts was not verified.
- **For calls *from* FreeSWITCH to the phone:** FreeSWITCH's offer must carry the fingerprint and ICE attributes. Without that, `setRemoteDescription` fails exactly as in the dart-sip-ua and 2015 reports.

### Gaps
- I found no 2025–2026 reports specific to webrtc-kmp + FreeSWITCH.
- I did not review dart-sip-ua's open issues for FreeSWITCH-specific bugs, because of the tool budget.
- I did not confirm FreeSWITCH's handling of an ICE restart in a re-INVITE.

## 4. TURN/STUN on mobile networks; trickle versus full gathering

### Takeaway
FreeSWITCH has no TURN client and generates no relay candidates. It does accept relay candidates from the client and falls back to them when the ACL drops everything else. Since FreeSWITCH normally sits on a public IP (ext-rtp-ip), a mobile client behind CGNAT usually connects with a srflx or even host candidate, because FreeSWITCH, as the full ICE agent with a public address, can reach it via peer-reflexive learning. A STUN server is still advisable, and TURN (UDP; TLS/443 for restrictive networks) is optional insurance. Trickle is not an option with mod_sofia, so use full gathering with a short timeout.

### Cited Findings
- FreeSWITCH: no relay candidates generated, client relay candidates accepted, no TURN client, only UDP candidates processed, STUN used only at profile load for ext-rtp-ip. — [FS docs](https://developer.signalwire.com/freeswitch/users-and-endpoints/webrtc-sip/)
- No trickle via mod_sofia (maintainer, Dec 2020). — [freeswitch-users](https://lists.freeswitch.org/pipermail/freeswitch-users/2020-December/134315.html)
- Kamailio users discussing the WebRTC control plane note that trickle ICE has to be bridged or avoided with SIP infrastructure (May 2020). — [sr-users 2020-05](https://lists.kamailio.org/pipermail/sr-users/2020-May/109102.html)
- Commercial SDKs such as Telnyx 3.8.0 (Nov 2026) make trickle optional (`setUseTrickleIce`), which shows that non-trickle is still the normal mode with SIP-style backends. — [Telnyx release note](https://telnyx.com/release-notes/webrtc-android-ice-restart)

### Inferences
- **Because FreeSWITCH ignores TCP candidates,** TURN over TCP/TLS only helps if the TURN server relays to FreeSWITCH over UDP. That is standard TURN behaviour: client↔TURN on TCP, TURN↔peer on UDP. The relay candidate itself is UDP-typed, so it should be accepted. This is inference.
- **Suggested client settings:**
  - `iceServers` = your STUN, plus optional TURN (udp + tls:443).
  - `iceCandidatePoolSize` of about 1–2, to pre-gather.
  - Gathering timeout around 1–2 s, with early release after the first srflx.
  - `continualGatheringPolicy` = GATHER_CONTINUALLY. It is available in webrtc-kmp v0.125.8+ and only matters for ICE restart after network changes.

### Gaps
- I found no measured data on how often mobile carriers need TURN when talking to a public-IP FreeSWITCH.
- I did not verify FreeSWITCH's behaviour with relay-only offers in practice.
