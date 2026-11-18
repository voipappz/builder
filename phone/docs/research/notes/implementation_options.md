# Implementation options for a native Kotlin/KMP softphone (SIP over WSS + WebRTC media to FreeSWITCH), November 2026

Scope note: partway through, the user added a constraint. The app must be easy to develop, so external native SIP libraries (baresip, PJSIP, Linphone through JNI/NDK) are out. Option 4 is reduced to a short exclusion note. Effort went into options 1, 2, 3 and 5 and the Flutter/React Native comparison.

Method: the repos were shallow-cloned on 2026-11-18. Line counts were measured locally: "NCLOC" means non-blank lines that are not comment lines (lines starting with `//`, `*` or `/*`), and "raw" means `wc -l`. Repo metadata (licence, stars, last push, releases) came from the GitHub API on the same day. FreeSWITCH `mod_verto.c` was fetched from signalwire/freeswitch master and grepped.

## Q1. What exists for each option: licence, work to a first call, pitfalls, maintenance (with ranking)

### Takeaway
Under the "easy to develop, Kotlin-first, permissive licence" constraints, the best options are:
- **Option 1+2 combined**: an own KMP SIP UA, ported structurally from sip.js's `core/` plus a slim API layer, running on webrtc-kmp.
- **Option 5**: mod_verto with a Telnyx-SDK-derived client. This is the fastest route to a first call on Android, but it ties the phone to Verto rather than SIP and needs a FreeSWITCH-side module and config.

Option 3 (sip.js inside QuickJS) is feasible but has no on-device precedent outside React Native, and it bridges a callback-heavy WebRTC API across two runtimes. Option 4 is excluded.

### Cited Findings

**Option 1/2: own KMP SIP UA + webrtc-kmp**
- webrtc-kmp is Apache-2.0 and supports Android, iOS and JS. It is built on WebRTC revision M125, and the iOS example pins the `WebRTC-SDK` pod 125.6422.05 — [shepeliev/webrtc-kmp](https://github.com/shepeliev/webrtc-kmp)
- webrtc-kmp activity: the latest release is v0.125.11 (2025-09-07). The last commit on the default branch is also 2025-09-03/07. There are open issues from 2026, including "Import WebRTC through SwiftPM instead of CocoaPods" (2026-10-26) and an iOS use-after-scope bug in `CValue.copyContents()` (2026-07-07), so nothing has been committed in about 14 months — GitHub API, `repos/shepeliev/webrtc-kmp` (queried 2026-11-18)
- webrtc-kmp has a common `DtmfSender` with Android, iOS and JS actuals (`src/commonMain/.../DtmfSender.kt`), so RFC 4733 DTMF over RTP is available — GitHub code search in shepeliev/webrtc-kmp (2026-11-18)
- Searches found no native Kotlin or Swift SIP-over-WebSocket (RFC 7118) UA library. Results returned only JS stacks (JsSIP, SIP.js), a React Native JsSIP fork, and a .NET project — [search results incl. JsSIP](https://github.com/versatica/JsSIP), [react-native-jssip](https://github.com/avodevelopment/react-native-jssip), [andikatjacobdennis/sip-over-websocket](https://github.com/andikatjacobdennis/sip-over-websocket)
- RFC 7118 defines the WebSocket transport for SIP ("SIP" WS subprotocol, Via transport WS/WSS) — [RFC 7118](https://www.rfc-editor.org/rfc/rfc7118.html)
- An alternative Android WebRTC binary exists: Google's own Android WebRTC library is described as no longer maintained, and GetStream's stream-webrtc-android and webrtc-sdk/android (MIT) are the prebuilt replacements — [GetStream/webrtc-android](https://github.com/GetStream/webrtc-android), [webrtc-sdk/android](https://github.com/webrtc-sdk/android)

**Option 2: sip.js as the porting reference** (detail in Q2)
- SIP.js is MIT ("Copyright (c) 2020 Junction Networks"). The last release on GitHub is 0.21.2 (2022-10-27) and the last push to main is 2026-06-15 (a docs/typo merge, PR #1127). The core code is effectively frozen and stable — local clone + GitHub API ([onsip/SIP.js](https://github.com/onsip/SIP.js))
- JsSIP is MIT (`"license": "MIT"` in package.json), at version 3.13.9. It is being actively migrated to TypeScript (last commit 2026-10-21, "TS: RequestSender (#991)"). Its src is 28,291 raw lines, of which 12,276 are the generated `Grammar.js` and 3,377 are `RTCSession.js` — local clone ([versatica/JsSIP](https://github.com/versatica/JsSIP))

**Option 3: sip.js/JsSIP in an embedded JS engine**
- quickjs-kt is Apache-2.0, at v1.0.18 (2026-11-07) and actively maintained. Its build declares `androidTarget`, `jvm`, `iosX64`, `iosArm64`, `iosSimulatorArm64`, macOS, Linux and mingw. It offers coroutine/suspend `evaluate()`, `asyncFunction()` bindings, ES modules with a custom loader and bytecode caching, and timeouts and interrupts — [dokar3/quickjs-kt](https://github.com/dokar3/quickjs-kt), build.gradle.kts, GitHub API
- Zipline (Cash App) is Apache-2.0, at 1.28.0 (2026-09-29). It embeds QuickJS in Kotlin/JVM and Kotlin/Native, precompiles JS to QuickJS bytecode, and works on Android 4.3+ and Kotlin/Native. It is designed around Kotlin/JS code calling Kotlin interfaces, not arbitrary npm libraries — [cashapp/zipline README](https://github.com/cashapp/zipline)
- React Native precedent: sip.js v0.16/v0.20 reportedly did not work out of the box with react-native-webrtc. One later user got sip.js ^0.21.2 working with react-native-webrtc ^124 and RN 0.68.2. A Stack Overflow answer says a custom SessionDescriptionHandler is needed — [SIP.js Google Group thread](https://groups.google.com/g/sip_js/c/ZUP0FEZpz44), [SO mirror](https://kiwix.lema.org/stackoverflow.com_en_all_2023-11/questions/50569000/integrate-sip-js-with-react-native)
- A JsSIP-on-react-native-webrtc user reported no remote streams on iOS, and no resolution was found — [react-native-webrtc discourse](https://react-native-webrtc.discourse.group/t/integrate-jssip-with-react-native-webrtc/284)
- Vendor SDKs ship this pattern commercially. The jambonz WebRTC SDK supports React Native on iOS and Android and uses JsSIP internally. The FreJun softphone SDK is built on sip.js + react-native-webrtc and needs PushKit + react-native-callkeep for iOS incoming calls — [jambonz docs](https://docs.jambonz.org/sdks/webrtc), [FreJun SDK listing](https://awesome.ecosyste.ms/projects/github.com%2Ffrejun-tech%2Freact-native-softphone-sdk)
- An Apple forum report says SIP.js WebRTC sessions worked when started manually but failed when driven through CallKit (an audio-session activation issue) — [Apple Developer Forums 691523](https://developer.apple.com/forums/thread/691523)
- App Store: Guideline 2.5.2 bars downloading or executing code that adds or changes functionality after review. The older 3.3.2 text carved out scripts run by WebKit/JavaScriptCore. Interpretations vary, and reviewers apply it inconsistently. These are secondary sources (forums and vendor blogs), not Apple's current text — [Capgo blog](https://capgo.app/blog/apple-remote-code-execution-capgo-live-updates/), [Apple forums 73640](https://developer.apple.com/forums/thread/73640), [Expo forum](https://forums.expo.dev/t/expo-updates-and-apples-app-store-review-guidelines/64495)

**Option 4: native SIP libraries (excluded by the user's constraint)**
- baresip is BSD-3-Clause and active (v4.13.0, 2026-11-04) — GitHub API ([baresip/baresip](https://github.com/baresip/baresip)). A Kotlin Android app (baresip-studio) wraps it through JNI — [GitHub topic sip-client](https://github.com/topics/sip-client)
- The licensing facts for PJSIP (GPL/commercial) and liblinphone (AGPL/commercial) were not re-verified here, because the option is excluded. They were already known to fail the "MIT/Apache/BSD only" rule in the brief.

**Option 5: mod_verto + Telnyx Android SDK**
- telnyx-webrtc-android is MIT ("Copyright (c) 2021 Telnyx") and very active (3.8.1 on 2026-11-05, push 2026-11-16). The iOS sibling telnyx-webrtc-ios is MIT (4.2.4 on 2026-11-12) — GitHub API, local clone ([team-telnyx/telnyx-webrtc-android](https://github.com/team-telnyx/telnyx-webrtc-android), [team-telnyx/telnyx-webrtc-ios](https://github.com/team-telnyx/telnyx-webrtc-ios))
- Telnyx `SocketMethod` enum: `telnyx_rtc.answer`, `.attach`, `.invite`, `.bye`, `.modify`, `.media`, `.info`, `.ringing`, `.ping`, `.clientReady`, `.gatewayState`, `.disable_push_notification`, `.attachCalls`, `.candidate`, `.endOfCandidates`, plus un-prefixed `login`, `anonymous_login` and `ai_conversation` — local clone, `telnyx_rtc/.../model/SocketMethod.kt`
- Stock FreeSWITCH mod_verto registers these JSON-RPC handlers: `login`, `verto.invite`, `verto.answer`, `verto.bye`, `verto.info`, `verto.modify`, `verto.attach`, `verto.ping`, `verto.subscribe`, `verto.unsubscribe`, `verto.broadcast`, `echo`, `fsapi`, `jsapi`. It sends `verto.media`, `verto.clientReady`, `verto.display`, `verto.punt`, `verto.event`, `verto.attach`, `verto.invite`, `verto.bye` and others to clients. `mod_verto.c` contains no `candidate`/trickle handler, no `gatewayState`, no `ringing` and no `login_token`. `verto.info` accepts a `dtmf` param (line ~3898) — `src/mod/endpoints/mod_verto/mod_verto.c` on signalwire/freeswitch master (7,093 lines), fetched 2026-11-18
- The Telnyx `LoginParam` sends `login_token` (Telnyx JWT), `login`, `passwd`, `userVariables`, `loginParams`, `sessid`, `User-Agent`, `mobile_sdk_name` and `mobile_sdk_version` — local clone, `verto/send/ParamRequest.kt`
- Telnyx SDK size: the `telnyx_rtc` module is about 17,900 raw Kotlin lines. `TelnyxClient.kt` is 4,580 lines, `Peer.kt` 2,690, `Call.kt` 777, `TxSocket.kt` 640 and `SdpUtils.kt` 614. Much of the rest is Telnyx-specific stats and reporting (`DebugDataCollector` 1,681, `WebRTCReporter` 936, `SignalingHealthMonitor` 807, `LatencyTracker` 604, `CallStatsUploader` 262) — local clone
- The Telnyx SDK depends on `com.telnyx.webrtc.lib:library:1.1.0` (Telnyx's own WebRTC build) and OkHttp — `telnyx_rtc/build.gradle` line 179
- FreeSWITCH docs describe mod_verto as "the Verto JSON-RPC WebRTC endpoint", with details in the "WebRTC with Verto" chapter — [SignalWire FreeSWITCH docs](https://developer.signalwire.com/freeswitch/FreeSWITCH-Explained/Modules/mod_verto_3964934/)
- signalwire/freeswitch is active (v1.11.3, 2026-08-28). GitHub reports its licence as NOASSERTION (it is MPL 1.1, which is server-side and does not affect the client) — GitHub API

**Flutter / React Native alternatives**
- dart-sip-ua is MIT ("Copyright (c) 2019 CloudWebRTC"), v1.1.0 (2025-12-10), last commit 2026-10-16. It is about 37,700 raw Dart lines and is a JsSIP port on flutter-webrtc — local clone + GitHub API ([flutter-webrtc/dart-sip-ua](https://github.com/flutter-webrtc/dart-sip-ua))
- react-native-webrtc is MIT, about 5k stars, release 155.0.0 (2026-10-14), so it now tracks a recent WebRTC milestone — GitHub API ([react-native-webrtc](https://github.com/react-native-webrtc/react-native-webrtc))

### Inferences
Ranking under the stated criteria (my synthesis):

| Rank | Option | Effort to first call (register, out/in call, 2-way audio, hang-up, DTMF) | Risk | Licence | Kotlin/KMP fit | Maintenance |
|---|---|---|---|---|---|---|
| 1 | **1+2: own KMP SIP UA modelled on/ported from sip.js core, on webrtc-kmp** | Medium: ~3–6 k Kotlin lines for the needed subset; about 3–6 engineer-weeks to a reliable first call against one known server (FreeSWITCH) | Medium: SIP edge cases (digest re-challenges, re-INVITE, CANCEL/487 races, WSS reconnect + re-REGISTER, Contact `;transport=ws` / `.invalid` host). webrtc-kmp is stale (no commits since Sep 2025, WebRTC M125) | MIT source to port + Apache-2.0 media: clean | Native, common code, coroutines | You own the stack, but SIP-over-WSS to a single FreeSWITCH is a small, stable surface. webrtc-kmp may need forking or swapping (stream-webrtc-android / WebRTC-SDK) |
| 2 | **5: mod_verto + Telnyx SDK fork** | Lowest on Android: rename `telnyx_rtc.*` to `verto.*`, drop trickle ICE, use login/passwd, strip stats/push/AI. Days to a couple of weeks | Medium-high: protocol drift from stock verto; enabling mod_verto plus verto.conf on FreeSWITCH changes the server side the brief targets (SIP over WSS); separate iOS SDK; no SIP features (REFER etc. come via FS) | MIT client; FreeSWITCH MPL server | Android-only Kotlin (not KMP); iOS is a separate Swift SDK | Fork divergence from a fast-moving upstream (weekly releases), and Telnyx's own WebRTC build |
| 3 | **3: sip.js in QuickJS (quickjs-kt) + native WebRTC bridge** | Medium-high: write a SessionDescriptionHandler and a WebSocket shim in Kotlin, plus timers/crypto polyfills. Fewer SIP lines, but two-runtime debugging | High: no precedent outside React Native; RN reports show SDH and iOS audio problems. Bundled (not downloaded) JS is how RN ships, so App Store risk is low if the JS is packaged at build | MIT/Apache: clean | Poor: Kotlin wraps a JS core | Upstream sip.js is frozen (fine), but you maintain the bridge |
| 4 | Flutter (dart-sip-ua) / RN (sip.js + react-native-webrtc) | Low for a demo; dart-sip-ua is a full JsSIP port | Medium: CallKit/ConnectionService and push glue still native; team switches language | MIT | None (non-Kotlin) | Active upstreams; the language change is the cost |
| — | 4: baresip/PJSIP/Linphone | Excluded by the user's constraint (native SIP libs, NDK builds); PJSIP/Linphone also fail on licence | | | | |

Notes behind the ranking:
- **Option 5 protocol delta (inference from the code diff above).** To talk to stock mod_verto, a Telnyx fork would need to:
  - rename the method prefix;
  - stop sending `telnyx_rtc.candidate`/`endOfCandidates` (stock has no trickle handler) and wait for ICE gathering to finish before sending the SDP in `verto.invite`/`verto.answer`;
  - authenticate with `login` + `passwd` (directory user) instead of `login_token`;
  - ignore or remove `ringing`, `gatewayState`, `attachCalls`, `disable_push_notification`, `anonymous_login` and `ai_conversation`;
  - handle stock-only server messages (`verto.display`, `verto.punt`, `verto.event`).

  DTMF through `verto.info {dtmf}` is supported by stock. The extra Telnyx messages are probably additions on Telnyx's own Verto-derived backend.
- **Option 5 bypasses the SIP-over-WSS path.** FreeSWITCH would need mod_verto enabled with a profile, and the phone would not be a SIP endpoint. That matters if other SIP features or servers are planned.
- **Pure SIP UA (option 1/2) keeps the server unchanged.** It works with any SIP-over-WSS server (mod_sofia `wss-binding`), and the same KMP core serves iOS later.
- **webrtc-kmp is the main dependency risk** (stale, M125). The fallback is a thin expect/actual over stream-webrtc-android (Android) and WebRTC-SDK via SwiftPM (iOS). That is the same thing webrtc-kmp does internally.

### Gaps
- I did not verify the licence of Telnyx's `com.telnyx.webrtc.lib:library:1.1.0` WebRTC binary. Upstream libwebrtc is BSD, but the redistribution terms of this build were not checked.
- There is no independent benchmark of QuickJS running sip.js on mobile (CPU, startup, battery).
- The current, official wording of App Store Guideline 2.5.2 was not fetched from apple.com. Only secondary sources were found.
- The engineer-week estimates are my judgement, not sourced.

## Q2. Which pieces of sip.js would be needed, and their line counts

### Takeaway
sip.js `src/` is 24,438 raw lines, or 13,384 NCLOC. A phone that only needs REGISTER, INVITE (in/out), CANCEL, BYE, ACK, digest auth and INFO/RFC 4733 DTMF needs roughly 8–9k NCLOC of it. That is about 4k for `core/` (transactions, dialogs, messages, user-agent-core, the needed UACs/UASs), about 3k for the API layer (UserAgent, Registerer, Inviter, Invitation, Session) and about 1.1k for the WebSocket transport + SDH. Most of this shrinks in Kotlin. SUBSCRIBE/PUBLISH/MESSAGE/REFER/PRACK and the 1,043-NCLOC SessionManager can be skipped.

### Cited Findings
All counts are from a local clone of onsip/SIP.js main (last commit 2026-06-15), measured 2026-11-18 — [onsip/SIP.js](https://github.com/onsip/SIP.js)
- Whole `src/`: 24,438 raw lines and 13,384 NCLOC. The largest raw files are `platform/web/session-manager/session-manager.ts` (1,603) and `api/session.ts` (1,464).
- `core/` NCLOC by directory:
  - `messages` 1,475, including `md5.ts` 364, `parser.ts` 213, `outgoing-request-message.ts` 203, `digest-authentication.ts` 128 and `utils.ts` 113
  - `user-agents` 1,293 (25 files): `user-agent-client` 236, `user-agent-server` 193, `invite-user-agent-client` 175, `subscribe-user-agent-client` 162, plus per-method UAC/UAS for bye, cancel, info, invite, message, notify, prack, publish, refer, register, re-invite, re-subscribe, subscribe
  - `dialogs` 1,135: `session-dialog` 537, `subscription-dialog` 351, `dialog` 228
  - `transactions` 965: `invite-client-transaction` 253, `invite-server-transaction` 240, `non-invite-client-transaction` 153, `non-invite-server-transaction` 147
  - `user-agent-core` 428 (`user-agent-core.ts` 369), `log` 135, `session` 68, `subscription` 30, `exceptions` 21, `timers.ts` 19
  - Raw `wc -l` per core dir: dialogs 2,194, messages 2,043, transactions 1,816, user-agents 2,353, user-agent-core 1,097.
- `grammar/`: TS wrappers are about 300 NCLOC (`uri.ts` 211). The parser itself is generated from `grammar/pegjs/src/grammar.pegjs` (1,009 lines), so the generated JS is not in `src/`.
- `api/` NCLOC (main files): `session.ts` 973, `user-agent.ts` 672, `inviter.ts` 569, `registerer.ts` 502, `invitation.ts` 485, `subscriber.ts` 340, `publisher.ts` 307, `user-agent-options.ts` 85, `subscription.ts` 83, `messager.ts` 61, `referral.ts` 61, plus many small option/state files.
- `platform/web/` NCLOC 2,821: `session-manager.ts` 1,043, `session-description-handler.ts` 631, `transport/transport.ts` 504, `simple-user.ts` 170, `modifiers.ts` 126. The SDH implements DTMF via RTP (RFC 4733), at `session-description-handler.ts` line ~325.
- For comparison: JsSIP src is 28,291 raw lines, of which `Grammar.js` (generated) is 12,276, `RTCSession.js` 3,377, `UA.js` 1,071, `SIPMessage.ts` 1,046 and `Transactions.ts` 869. dart-sip-ua lib is 37,676 raw Dart lines — local clones ([JsSIP](https://github.com/versatica/JsSIP), [dart-sip-ua](https://github.com/flutter-webrtc/dart-sip-ua))

### Inferences
Needed-for-a-phone subset (my estimate from the NCLOC above):

| Area | Needed sip.js files | Approx. TS NCLOC |
|---|---|---|
| Messages + parser + digest | `messages/*` minus `md5.ts` (Kotlin/platform crypto replaces it) | ~1,100 |
| Grammar | a hand-written parser for Via/From/To/Contact/CSeq/WWW-Authenticate/URI instead of pegjs | ~500–800 Kotlin |
| Transactions | all 4 (RFC 3261 §17), plus timers | ~985 |
| Dialogs | `dialog` + `session-dialog` (skip subscription-dialog) | ~780 |
| User-agent core | `user-agent-core` + base UAC/UAS | ~860 |
| Per-method UAs | invite UAC/UAS, re-invite, bye, cancel, register, info | ~500 |
| API | UserAgent, Registerer, Inviter, Invitation, Session (minus refer/message) | ~2,800 |
| Transport | WebSocket (RFC 7118 framing, keepalive, reconnect) | ~500 |
| SDH | glue to webrtc-kmp (offer/answer, hold, DTMF) | ~600 |
| **Total** | | **~8–9k TS NCLOC, likely 5–7k Kotlin** |

- Because transport over WSS is reliable, the hardest RFC 3261 timer logic (retransmission timers A/E/G) is mostly inert. Timer B/F timeouts and the INVITE 2xx/ACK handling still matter.
- For a first call only, a simpler design is possible: a single-dialog state machine without the full sip.js layering (~2–3k Kotlin). This trades structure for speed, and sip.js's layering is what makes re-INVITE, glare and CANCEL races tractable later.

### Gaps
- NCLOC uses a simple regex (it skips lines starting with `//`, `*` or `/*`). It does not count tokens or statements.
- The size of the generated pegjs parser in the published sip.js build was not measured.

## Q3. Precedent for teams writing their own SIP UA in Kotlin/Swift for production

### Takeaway
I found no public, open-source native Kotlin or Swift SIP-over-WebSocket UA used in production. The production mobile WebRTC softphone SDKs found take one of two routes:
- **Their own JSON-RPC signalling instead of SIP**: Telnyx with Verto-derived `telnyx_rtc.*`.
- **A JS SIP stack inside React Native**: jambonz (JsSIP) and FreJun (sip.js).

Building a native SIP UA is therefore less trodden ground, but its scope is bounded.

### Cited Findings
- Telnyx's production Android and iOS SDKs are native (Kotlin/Swift) but speak Verto-derived JSON-RPC over WSS rather than SIP — local clone of [telnyx-webrtc-android](https://github.com/team-telnyx/telnyx-webrtc-android) (`SocketMethod.kt`)
- jambonz's mobile SDK uses JsSIP inside React Native, and FreJun's uses sip.js + react-native-webrtc — [jambonz docs](https://docs.jambonz.org/sdks/webrtc), [FreJun SDK](https://awesome.ecosyste.ms/projects/github.com%2Ffrejun-tech%2Freact-native-softphone-sdk)
- Searches for a Kotlin or Swift RFC 7118 UA library surfaced only JS libraries, an RN fork of JsSIP and a .NET project — [react-native-jssip](https://github.com/avodevelopment/react-native-jssip), [sip-over-websocket (.NET)](https://github.com/andikatjacobdennis/sip-over-websocket)
- Existing native Android SIP apps (Linphone Android, baresip-studio) are Kotlin UIs over C SIP stacks through JNI — [GitHub topic sip-client](https://github.com/topics/sip-client), [GitHub topic voip (Kotlin)](https://github.com/topics/voip?l=kotlin)

### Inferences
- The absence of a reusable native SIP-over-WSS library is itself evidence that option 1/2 means writing code nobody else maintains. The upside is that the needed subset is well specified (RFC 3261 + 7118 + 2617/7616 digest + 4733 DTMF) and sip.js's MIT core is a proven blueprint to port against.
- Commercial vendors probably have closed native SIP stacks (e.g. in UCaaS apps), but nothing found is citable.

### Gaps
- There are no public conference talks or blog posts in the results about building a SIP UA from scratch in Kotlin or Swift. The searches were limited (about 15 tool calls), so a deeper search of KotlinConf, ClueCon and Kamailio World talks might find some.
