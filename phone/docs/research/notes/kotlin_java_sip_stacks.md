# Kotlin / Java / KMP SIP user-agent stacks usable on Android and iOS (as of 2026-11-18)

Method: GitHub metadata via `gh api` (licence SPDX, stars, `pushed_at`) on 2026-11-18. Shallow clones of uvp-gb28181-sim, sip4k, gsm2sip, usnistgov/jsip and webphone-android, read directly. GitHub code search for Kotlin files containing `Sec-WebSocket-Protocol`, `transport=ws` and `SIP/2.0/WSS`. Maven Central search API. Each "Verified" item below was read in code or in repo metadata. Inferences are marked as such.

## Q1. Which Kotlin, KMP or Java SIP stacks/UAs exist, and what do they offer?

### Takeaway
No mature, permissively licensed SIP UA covers Kotlin Multiplatform with iOS. The only mature permissive Java stack is NIST JAIN-SIP (public domain/NIST notice). It is JVM/Android only, has not had a Maven release since 2018, and is heavy. Everything else is one of three things: copyleft (Sipdroid, mjSIP, CSipSimple, Lumicall, phhusson/ims, imsdroid, Linphone, PJSIP, RestComm SDK), dead (sip4k, imsdroid, CSipSimple), or a small hand-rolled Kotlin UA inside an app. Several of those small app UAs are MIT/Apache, and some are written for FreeSWITCH over WSS.

### Cited Findings

**Inventory table** (stars and last push are GitHub API values on 2026-11-18)

| Project | Lang | Licence (exact) | Stars | Last push | Transports | Mobile / KMP | Verdict |
|---|---|---|---|---|---|---|---|
| NIST JAIN-SIP RI (usnistgov/jsip) | Java | NIST public-domain notice (GitHub: NOASSERTION) | 318 | 2024-07-08 | UDP, TCP, TLS, SCTP, WS, WSS (NIO) | Android build existed; JVM only, no iOS | Permissive and complete RFC 3261 stack, but stale, heavy and log4j-era |
| RestComm/jain-sip (fork/mirror) | Java | none detected by GitHub (inherits NIST notice) | 176 | 2024-01-09 | same as above | same | Mirror, stale |
| Maven `javax.sip:android-jain-sip-ri` | Java | NIST | – | latest 1.3.0-91, published 2018-03 | – | Android | Last artifact 2018 |
| uvp-gb28181-sim (Frank-Coding97) | Kotlin MP | MIT | 12 | 2026-11-18 | UDP, TCP (Ktor sockets); no TLS/WS | KMP: Android + iOS + JVM | Best KMP code to copy pieces from; GB28181 device (UAS) role |
| sip4k (soft-stech) | Kotlin JVM | MIT | 9 | 2021-12-24 | UDP (Netty) | Server-oriented | Dead, minimal |
| gsm2sip (pulpoff) | Kotlin Android | MIT | 182 | 2026-11-04 | UDP, TLS | Android only | Small hand-rolled UA, actively maintained |
| webphone-android (LarsArtmann) | Kotlin Android | MIT | 0 | 2026-11-15 | **WSS** (OkHttp) | Android only | Closest functional match (FreeSWITCH + WSS + WebRTC); very new, partly untested |
| nwaycn/nsbc Android SDK | Kotlin Android | Apache-2.0 | 12 | 2026-11-18 | WS/WSS (inferred from code search hit) | Android only | Worth a read; not inspected in depth |
| eddyslarez/sip-library-android | Kotlin Android | README badge says MIT, but the repo has **no LICENSE file** | 0 | 2025-10-07 | WSS + WebRTC (README) | Android | Licence unclear; treat as all-rights-reserved |
| OrhanYigitDurmaz/h2open-mobile `sip/` | Kotlin | **no licence** | 0 | 2026-10-20 | WS (code-search hit) | ? | Not usable without a licence |
| Sipdroid (i-p-tel/sipdroid) | Java | GPL-3.0 | 699 | 2026-07-06 (v7.0 commit 2025-12-01) | UDP, TCP, TLS (mjSIP-based) | Android | GPL; excluded |
| mjSIP (haumacher/mjSIP) | Java | GPL-2.0 | 65 | 2026-08-08 | UDP, TCP, TLS | JVM | GPL; excluded |
| Lumicall (opentelecoms-org) | Java | GPL-3.0 | 155 | 2025-08-25 | mjSIP fork + ICE/TLS | Android | GPL; excluded |
| CSipSimple (r3gis3r mirror) | Java + PJSIP | LGPL-3.0 per GitHub (app is GPL; native PJSIP is GPL-2.0) | 311 | 2016-03-11 | via PJSIP | Android | Dead and copyleft |
| phhusson/ims | Kotlin | GPL-2.0 | 186 | 2025-01-15 | IMS over IPsec TCP/UDP | Android (system app) | GPL, IMS-specific |
| Doubango imsdroid | Java + C (tinySIP) | GitHub NOASSERTION (Doubango is GPLv3/commercial) | 304 | 2019-10-24, **archived** | doubango had WS/WSS | Android | Dead, copyleft |
| RestComm Android SDK | Java (JAIN-SIP + libwebrtc) | AGPL-3.0 | 160 | 2020-02-01 | TCP/TLS (WSS commented out) | Android | AGPL and dead |
| RestComm iOS SDK | ObjC (sofia-sip) | AGPL-3.0 | 39 | 2018-11-16 | – | iOS | AGPL and dead |
| Linphone Android | Kotlin over C (liblinphone) | GPL-3.0 (commercial available) | 1341 | 2026-11-14 | UDP/TCP/TLS | Android/iOS | GPL or paid |
| PJSIP (pjsua2 Kotlin binding) | C + Kotlin/Java wrapper | GPL-2.0 (commercial available) | 2709 | 2026-11-18 | UDP/TCP/TLS | Android/iOS | GPL or paid; no WS client in pjsip |
| baresip-studio | Kotlin over C baresip | BSD-3-Clause | 441 | 2026-11-17 | UDP/TCP/TLS (+baresip WS module?) | Android | Permissive, but the stack is C, not Kotlin |
| SIPRAL | Rust with Kotlin bindings | AGPL-3.0 or commercial | 5 | 2026-11-17 | – | multi | AGPL; excluded |
| Telnyx WebRTC Android | Kotlin | MIT | 25 | 2026-11-16 | Verto-style JSON-RPC over WSS, **not SIP** | Android | Not SIP |

Sources:
- uvp-gb28181-sim: MIT, 12★, pushed 2026-11-18. KMP Android/iOS. — [GitHub](https://github.com/Frank-Coding97/uvp-gb28181-sim)
- NIST jsip: 318★, pushed 2024-07-08. LICENSE says NIST works are "not subject to copyright protection in the United States" and grants permission to use, copy, modify and distribute. — [usnistgov/jsip](https://github.com/usnistgov/jsip), [LICENSE](https://github.com/usnistgov/jsip/blob/master/LICENSE)
- RestComm/jain-sip: 176★, pushed 2024-01-09. Described as a git-svn mirror of NIST jsip with TeleStax productization. — [RestComm/jain-sip](https://github.com/RestComm/jain-sip)
- Maven Central: `javax.sip:android-jain-sip-ri` and `javax.sip:jain-sip-ri` latest 1.3.0-91, timestamped March 2018. — [Maven Central search](https://search.maven.org/search?q=g:javax.sip)
- jsip ships `src/examples/android/simplecallsetup`. Its README says "To convert any example to android you simply put android prefix to the JAIN SIP packages" and points to a CloudBees Android build. — [jsip examples/android](https://github.com/usnistgov/jsip/tree/master/src/examples/android)
- sip4k README: "Lock-free coroutine-based implementation of sip and rtp protocols… g711… Designed for server application development". Last commit 2021-12-24 ("Fix register npe"). — [soft-stech/sip4k](https://github.com/soft-stech/sip4k)
- gsm2sip: MIT, last push 2026-11-04. About 2.5k lines of hand-written Kotlin SIP (SipClient 993, SipMessage 671, SipCall 486, SipTransport 300, SipAuth 81). Transports are UDP (`Os.socket` DGRAM) and TLS (`SSLSocketFactory`), with no WS. Handles INVITE Timer A retransmission, 401/407 re-auth, in-dialog re-INVITE/UPDATE answering, and DTMF via SIP INFO (dtmf-relay) plus RTP. Unit tests cover only RTP/DTMF codecs. — [pulpoff/gsm2sip](https://github.com/pulpoff/gsm2sip)
- Sipdroid: GPL-3.0, 699★. Latest commit 2025-12-01 ("7.0 … Improved Bluetooth support"), repo pushed 2026-07-06. — [i-p-tel/sipdroid](https://github.com/i-p-tel/sipdroid)
- mjSIP: GPL-2.0, 65★, pushed 2026-08-08. README says it is "available open source under the terms of the GNU GPL" and is not on Maven Central (GitHub Packages, auth required). — [haumacher/mjSIP](https://github.com/haumacher/mjSIP)
- Lumicall: README "Licensed under the GPL v3". 155★, pushed 2025-08-25. Maven `org.opentelecoms.sip:mjsip-fork` latest 1.6+lumicall.3 (2016). — [opentelecoms-org/lumicall](https://github.com/opentelecoms-org/lumicall)
- CSipSimple mirror: LGPL-3.0 per GitHub, 311★, last push 2016-03-11. — [r3gis3r/CSipSimple](https://github.com/r3gis3r/CSipSimple)
- phhusson/ims: GPL-2.0, 186★, pushed 2025-01-15. Kotlin `me.phh.sip` (SipConnection, SipHandler, SipMessage, SipReader, SipChallenge) with unit tests for the message parser/reader. — [phhusson/ims](https://github.com/phhusson/ims)
- DoubangoTelecom/imsdroid: archived, last push 2019-10-24. DoubangoTelecom/doubango: archived, 2019-07-20. — [imsdroid](https://github.com/DoubangoTelecom/imsdroid), [doubango](https://github.com/DoubangoTelecom/doubango)
- RestComm Android SDK: AGPL-3.0, last push 2020-02-01. Depends on `javax.sip:android-jain-sip-ri:1.2.293`, `org.mobicents.javax.sip:android-jain-sip-ext:1.3.33` and `org.webrtc:google-webrtc:1.0.21217`. — [restcomm-android-sdk](https://github.com/RestComm/restcomm-android-sdk), [build.gradle](https://github.com/RestComm/restcomm-android-sdk/blob/master/restcomm.android.sdk/build.gradle)
- RestComm iOS SDK: AGPL-3.0, last push 2018-11-16. — [restcomm-ios-sdk](https://github.com/RestComm/restcomm-ios-sdk)
- Linphone Android: GPL-3.0, 1341★, pushed 2026-11-14. — [linphone-android](https://github.com/BelledonneCommunications/linphone-android)
- PJSIP: GPL-2.0, 2709★. Kotlin example in the docs. — [pjproject](https://github.com/pjsip/pjproject), [PJSIP Kotlin example](https://docs.pjsip.org/en/2.17/get-started/android/kotlin-sip-client.html)
- baresip-studio: BSD-3-Clause, 441★, pushed 2026-11-17. — [juha-h/baresip-studio](https://github.com/juha-h/baresip-studio)
- SIPRAL: "AGPL-3.0 or commercial". Rust SIP stack with Kotlin/Swift bindings, created 2026-09-04. — [SIPRAL/sipral](https://github.com/SIPRAL/sipral)
- Telnyx WebRTC Android SDK: MIT, pushed 2026-11-16. Telnyx-specific WebRTC signalling. — [telnyx-webrtc-android](https://github.com/team-telnyx/telnyx-webrtc-android)
- "KotlinExtension.SIP": no project by this name found on GitHub or the web. — [web search results](https://developer.android.com/reference/kotlin/android/net/sip/SipManager) (only unrelated hits)

### Inferences
- No candidate is a "drop-in KMP SIP UA library under MIT/Apache with iOS support". For Kotlin Multiplatform, the realistic path is to write a small SIP UA in commonMain. Templates to copy from: uvp-gb28181-sim (KMP structure, parser, digest, tests) and webphone-android (WSS + WebRTC call flows against FreeSWITCH).
- JAIN-SIP could serve an Android-only build: it has a WS client channel and a full transaction/dialog layer. It cannot run on iOS (JVM-only; no Kotlin/Native path), and its WS client is crude (see Q2).
- In terms of RFC 3261 features, the small hand-rolled UAs do not implement a proper transaction layer (§17) for reliable transports. Over WSS this matters less, because RFC 3261 §17 retransmission timers A/E are not used on reliable transports.

### Gaps
- Exact feature matrices (ICE, REFER, RFC 4733) for Sipdroid, mjSIP and Lumicall were not re-verified in code. They are excluded on licence anyway.
- The nwaycn/nsbc Android SDK was only located via a code-search hit (`Sec-WebSocket-Protocol` in `sip/SipTransport.kt`). Its feature coverage and quality were not inspected.
- Not verified whether baresip-studio enables baresip's WebSocket transport.

## Q2. Does any Kotlin/Java SIP UA support SIP over WebSocket as a client (RFC 7118)?

### Takeaway
Yes, but only in two places. NIST JAIN-SIP has WS/WSS channels with a client mode, and that client handshake is a hack. A handful of 2025–2026 Kotlin Android apps implement SIP over WSS natively with OkHttp: webphone-android (MIT, targets FreeSWITCH), nsbc SDK (Apache-2.0), and eddyslarez/sip-library-android (no licence file). None of them is KMP/iOS.

### Cited Findings
- jsip (and the RestComm mirror) contain `NioWebSocketMessageChannel`, `NioTlsWebSocketMessageChannel`, `NioWebSocketMessageProcessor`, `NioTlsWebSocketMessageProcessor`, `WebSocketCodec` and `WebSocketHttpHandshake`, plus a `src/examples/websocket/B2BUA.java` example ("This is a Websocket B2BUA server"). — [jsip stack sources](https://github.com/usnistgov/jsip/tree/master/src/gov/nist/javax/sip/stack)
- **Verified in code:** in client mode, `NioWebSocketMessageChannel` sends the HTTP Upgrade before the first request. It uses a hardcoded `Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==` (the RFC 6455 sample nonce), takes Host/Location/method from headers embedded in the Request-URI, then does `Thread.sleep(500)` instead of waiting for the 101 response. — [NioWebSocketMessageChannel.java](https://github.com/usnistgov/jsip/blob/master/src/gov/nist/javax/sip/stack/NioWebSocketMessageChannel.java)
- The RestComm Android SDK (JAIN-SIP + WebRTC) uses `transport = "tcp"` or `"tls"`. `//transport = "wss";` is commented out in JainSipClient.java, so even RestComm's own mobile WebRTC SDK did not ship JAIN-SIP over WSS. — [JainSipClient.java](https://github.com/RestComm/restcomm-android-sdk/blob/master/restcomm.android.sdk/src/main/java/org/restcomm/android/sdk/SignalingClient/JainSipClient/JainSipClient.java)
- webphone-android (MIT, created 2026-10-25, 203 commits, last push 2026-11-15). README: "Call signaling (SIP over WSS: REGISTER, INVITE, hold re-INVITE, BYE, DTMF `application/dtmf-relay` equals-form, REFER + NOTIFY sipfrag) — `sip/SipEngine.kt`". Audio is WebRTC via stream-webrtc-android, with Telecom self-managed ConnectionService. — [webphone-android](https://github.com/LarsArtmann/webphone-android)
  - Verified: SipEngine.kt (869 lines) uses `OkHttpClient.newWebSocket`. Its header comment notes that the DTMF INFO colon form "is 200-OK'd by FreeSWITCH and silently dropped", and that a hold re-INVITE re-presents the same SDP so ICE is not renegotiated. Digest.kt (94 lines) handles qop lists and SHA-256. Tests: SipMessageTest, DigestTest (RFC 2617 §3.5 vector), SipEngineTest, SipEngineAuthTest, SipEngineOverTheWireTest and SipEngineSilenceTest, about 1,070 test lines in total.
  - README honesty markers: REGISTER/INVITE/BYE/DTMF/hold are "implemented, **not yet exercised against the stack's WSS gateway**", and incoming INVITE and REFER/NOTIFY are "**explicitly UNTESTED** against live FreeSWITCH".
- nwaycn/nsbc includes `client/android/nsbc-sdk/.../sip/{SipEngine,SipTransport,SipMessage,DigestAuth,SipUtils}.kt`. SipTransport.kt matched a code search for `Sec-WebSocket-Protocol` (sip subprotocol). The repo LICENSE is Apache-2.0 (© 2024-2026 Li Hao / 上海宁卫信息技术有限公司). — [nwaycn/nsbc](https://github.com/nwaycn/nsbc)
- eddyslarez/sip-library-android: README (Spanish) describes a SIP/VoIP library for Android "usando WebRTC y WebSocket con soporte multi-cuenta", with `webSocketUrl = "wss://…"`. It shows an MIT badge, but the repo root has no LICENSE file (GitHub licence: none). 0★, last push 2025-10-07. — [eddyslarez/sip-library-android](https://github.com/eddyslarez/sip-library-android)
- Other Kotlin hits for `SIP/2.0/WSS` or `transport=ws`: vayun-mathur/Modern-Apps (GPL-3.0, Google Voice SIP), OrhanYigitDurmaz/h2open-mobile (no licence) and ratulhasanruhan/ipcallbd (no licence). — [Modern-Apps](https://github.com/vayun-mathur/Modern-Apps), [h2open-mobile](https://github.com/OrhanYigitDurmaz/h2open-mobile), [ipcallbd](https://github.com/ratulhasanruhan/ipcallbd)
- RFC 7118 defines the WS/WSS Via transports and the `transport=ws` URI parameter (there is no `wss` URI parameter value). — [RFC 7118](https://www.rfc-editor.org/info/rfc7118)

### Inferences
- In code, SIP over WSS in Kotlin is a thin layer: one OkHttp/Ktor WebSocket with subprotocol `sip`, one SIP message per text frame, and no framing or Content-Length parsing. The hard parts are dialog/transaction state, digest, and SDP/ICE handling with WebRTC. webphone-android shows the scope is about 1k lines for a FreeSWITCH-targeted UA.
- For KMP, Ktor client WebSockets (Android: OkHttp/CIO engine; iOS: Darwin engine) would replace OkHttp. uvp-gb28181-sim already uses Ktor 3.0.2 (`ktor-network`, `ktor-client-darwin`). This is an inference, not tested.

### Gaps
- No KMP (iOS-capable) Kotlin SIP-over-WSS client was found anywhere.
- No production-maturity evidence (store apps, user base) was found for any of the Kotlin WSS UAs.

## Q3. uvp-gb28181-sim shared SIP code: quality, tests, reusability

### Takeaway
High-quality, heavily tested KMP code (Android/iOS/JVM, MIT, very active), but written for a GB/T 28181 camera device. It mostly acts as a UAS for INVITE, carries GB28181-specific SDP (`y=`, `f=`), has no RFC 3261 transaction layer or retransmission, no WS/TLS, MD5-only digest, and no ICE. Reusable parts: the parser/serializer, the header helpers, the digest skeleton, the `SipTransport` interface and Flow-based design, the dialog-identity checks and the test style. The INVITE/SDP flows would need rewriting for a WebRTC UAC.

### Cited Findings (all verified by reading the clone, HEAD 4eb9f90 "chore(release): v1.0.15", 2026-11-18)
- History: first commit in the shallow-50 window is 2026-10-21, and multiple releases happened on 2026-11-18 (v1.0.13 to v1.0.15). — [commits](https://github.com/Frank-Coding97/uvp-gb28181-sim/commits)
- LICENSE: "MIT License, Copyright (c) 2026 UVP". — [LICENSE](https://github.com/Frank-Coding97/uvp-gb28181-sim/blob/main/LICENSE)
- `shared/src/commonMain/kotlin/com/uvp/sim/sip/` has 29 files totalling 3,559 lines: SipParser, SipSerializer, SipMessage, SipHeader(s), SipHeaderHelpers, SipMethod, DigestAuth, Md5 (pure Kotlin), Sdp, SdpPlaybackParser, SipBuilders/SipInviteBuilders/SipRegisterBuilders/SipResponseBuilders, SipDialogIdentityService, DialogIdentityVerifier, SipDialogIdleTracker, SipMessageRouter, SipOutbox, SubscribeHandler, plus GB-specific files (MansRtspParser, GbVersionNegotiation, …). — [sip/](https://github.com/Frank-Coding97/uvp-gb28181-sim/tree/main/shared/src/commonMain/kotlin/com/uvp/sim/sip)
- `SipMethod` enum covers REGISTER, MESSAGE, INVITE, ACK, BYE, SUBSCRIBE, NOTIFY, OPTIONS, CANCEL and INFO. There is **no REFER, UPDATE or PRACK**. — same dir, SipMethod.kt
- `network/`: `SipTransport` interface (`connect`, `send`, `incoming: Flow<SipEnvelope>`, `close`, `localPort`, `isConnected`, plus a passive-disconnect signal) implemented by `UdpSipTransport` and `TcpSipTransport` (473 lines, Ktor `io.ktor.network.sockets`). No TLS or WebSocket transport. RTP sender/receiver and NetworkController are `expect` classes with android/ios/jvm actuals. — [network/](https://github.com/Frank-Coding97/uvp-gb28181-sim/tree/main/shared/src/commonMain/kotlin/com/uvp/sim/network)
- `SipOutbox` doc comment (translated): "does not implement RFC3261 §17 retransmission (D1) — the simulator relies on underlying UDP + peer application-layer retry". So there are no client/server transaction state machines. — SipOutbox.kt
- `DigestAuth`: RFC 2617 + RFC 3261 §22.4, parses `WWW-Authenticate`, handles qop=auth (first option) and opaque, and throws on anything but MD5 (`"Only MD5 algorithm is supported"`). It was written for REGISTER 401. — DigestAuth.kt
- `Sdp.kt` (285 lines): `SdpOffer` with direction (sendrecv/sendonly/recvonly/inactive), media VIDEO/AUDIO, transport UDP/TCP with `setup` active/passive/actpass, and GB-specific builders (`buildPlayAnswer`, `buildBroadcastOffer`, SSRC `y=` line, `f=` line). There is no ICE, DTLS fingerprint, rtcp-mux or BUNDLE handling. — Sdp.kt
- `domain/coord/InviteDialogHandler.kt` (245 lines): the mid-dialog handler for an inbound live-view INVITE. It handles ACK/CANCEL/BYE, dialog identity verification (returns 481 per RFC 3261 §12.2.1.1) and a 32 s ACK watchdog ("SIP T1 * 64, RFC 3261 §17.1.1.2"), with Mutex-guarded coroutine state. Comments are a mix of English and Chinese and refer to internal review rounds ("cross-review R3", "RG-1"). — [InviteDialogHandler.kt](https://github.com/Frank-Coding97/uvp-gb28181-sim/blob/main/shared/src/commonMain/kotlin/com/uvp/sim/domain/coord/InviteDialogHandler.kt)
- Tests: 267 `*Test.kt` files in `shared/src` (commonTest, jvmTest, iosTest). The `sip/` commonTest files alone contain 307 `@Test` methods (SipParserTest, DigestAuthTest, SipHeaderHelpersTest, SipComplianceTest, SipMessageRouterTest, PlatformAuthorizerTest, SubscribeHandlerTest, SipDialogIdentityServiceTest, …). There are also jvmTest transport tests (UdpSipTransportTest, TcpSipTransportReconnectTest, SipTransportEnvelopeTest). — [shared/src](https://github.com/Frank-Coding97/uvp-gb28181-sim/tree/main/shared/src)

### Inferences
- Copy as-is (with attribution, MIT): SipParser/SipSerializer/SipMessage/SipHeaderHelpers (multiplatform, byte-array based, handles header folding and Content-Length), Md5, the DigestAuth structure (add SHA-256 and the 407/Proxy-Authorization path), and the `SipTransport` + `Flow<SipEnvelope>` abstraction, which a `KtorWebSocketSipTransport` could implement.
- Rewrite: INVITE handling (a softphone is mainly a UAC that sends INVITE with WebRTC SDP and also answers), SDP (WebRTC SDP should be passed through from libwebrtc rather than built), REFER/hold/INFO-DTMF, and the transaction layer (minimal over WSS: CSeq/branch matching and Timer B/F timeouts).
- Discard: the GB28181 domain (MANSCDP, catalog, playback, broadcast) is about 60% of the SIP-adjacent code.
- Quality is high for a 1-month-old project: extensive tests, explicit RFC references, coroutine-safe design. Some risk comes from fast churn (three releases in one day) and Chinese-language design notes.

### Gaps
- Did not run the test suite or measure line coverage.
- Did not check how strictly the parser handles compact header forms or multi-value headers relative to RFC 3261 §7.3.

## Q4. Reusable Kotlin SIP message parsers (e.g. sip3io)

### Takeaway
sip3's Kotlin SIP parsing (Apache-2.0, active) is a thin wrapper over NIST JAIN-SIP's `StringMsgParser`. It is JVM-only and not usable in KMP commonMain. The reusable pure-Kotlin multiplatform parser is uvp-gb28181-sim's `SipParser`. webphone-android's `SipMessage.kt` (97 lines, MIT) is a minimal Android codec.

### Cited Findings
- sip3-commons: Apache-2.0, 59★, pushed 2026-11-14, version 2026.2.2-SNAPSHOT. It contains SIP method constants, SdpSession and payload types, but no SIP message parser. — [sip3-commons](https://github.com/sip3io/sip3-commons)
- sip3-salto-ce (Apache-2.0) `SipMessageParser.kt` imports `gov.nist.javax.sip.parser.*` and `gov.nist.javax.sip.message.SIPMessage`, and subclasses `StringMsgParser`. Its pom depends on `javax.sip:jain-sip-ri`. — [SipMessageParser.kt](https://github.com/sip3io/sip3-salto-ce/blob/master/src/main/kotlin/io/sip3/salto/ce/sip/SipMessageParser.kt)
- uvp-gb28181-sim `SipParser.parse(raw: ByteArray): SipMessage` handles continuation-line folding, Content-Length and the header/body boundary, and throws `SipParseException`. It sits in commonMain and is tested by SipParserTest (220 lines). — [SipParser.kt](https://github.com/Frank-Coding97/uvp-gb28181-sim/blob/main/shared/src/commonMain/kotlin/com/uvp/sim/sip/SipParser.kt)
- phhusson/ims has a Kotlin SipReader/SipMessage parser with tests, but it is GPL-2.0. — [phhusson/ims](https://github.com/phhusson/ims)

### Inferences
- If the target only needs Android, NIST `StringMsgParser` (public domain) is a robust option. For KMP, use or adapt the uvp-gb28181-sim parser.

### Gaps
- Did not find a standalone, published (Maven Central) pure-Kotlin SIP parser library.
