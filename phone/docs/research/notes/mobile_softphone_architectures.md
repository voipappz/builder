# Mobile softphone architectures and permissively licensed reusable code (as of 2026-11-18)

Method note: licence and structure facts for GitHub repos were read straight from the GitHub API on 2026-11-18 (repo `license.spdx_id`, LICENSE files, the file tree, build.gradle). Vendor protocol facts come from vendor docs. Where something is my inference, it is listed under "Inferences".

## 1. Vendor SDKs and apps: signalling, media, licence, push

### Takeaway
None of the big CPaaS mobile SDKs speak plain SIP over WSS to the device. Telnyx uses a Verto-derived JSON-RPC over a secure socket. Twilio uses a proprietary TLS signalling channel. Vonage, Sinch and SignalWire all use their own SDK protocols. All of them use libwebrtc for media. Telnyx is the only one whose whole native SDK (core plus app layer plus samples) is MIT on GitHub. Twilio's quickstarts are MIT and its RN SDK is Apache-2.0, but the native Voice SDK binaries are closed. The SIP-native mobile products (Linphone, Acrobits, Zoiper, 3CX) use SIP over UDP/TCP/TLS, not WSS. They are either GPL/AGPL (Linphone) or commercial.

### Cited Findings
**Telnyx (Android/iOS)**
- The Android repo `team-telnyx/telnyx-webrtc-android` is MIT ("MIT License, Copyright (c) 2021 Telnyx"). The iOS repo `team-telnyx/telnyx-webrtc-ios` is also MIT per GitHub metadata. The latest releases are Android 3.8.1 (2026-11-05) and iOS 4.2.4 (2026-11-12). — [GitHub telnyx-webrtc-android](https://github.com/team-telnyx/telnyx-webrtc-android); [GitHub telnyx-webrtc-ios](https://github.com/team-telnyx/telnyx-webrtc-ios)
- Signalling is a Verto-style JSON-RPC protocol with Telnyx-renamed methods. `SocketMethod.kt` defines `telnyx_rtc.invite`, `telnyx_rtc.answer`, `telnyx_rtc.bye`, `telnyx_rtc.modify`, `telnyx_rtc.media`, `telnyx_rtc.info`, `telnyx_rtc.ringing`, `telnyx_rtc.attach`, `telnyx_rtc.candidate`, `telnyx_rtc.endOfCandidates`, `telnyx_rtc.gatewayState`, `telnyx_rtc.disable_push_notification`, `login` and `anonymous_login`. The production host in `Config.kt` is `rtc.telnyx.com`. The README calls the answer message a "verto payload". — [SocketMethod.kt](https://github.com/team-telnyx/telnyx-webrtc-android/blob/main/telnyx_rtc/src/main/java/com/telnyx/webrtc/sdk/model/SocketMethod.kt); [README](https://github.com/team-telnyx/telnyx-webrtc-android)
- Media comes from a Telnyx-precompiled libwebrtc artifact, `com.telnyx.webrtc.lib:library:1.1.0`. The transport stack is OkHttp, Gson and coroutines (from `telnyx_rtc/build.gradle`). — [telnyx_rtc/build.gradle](https://github.com/team-telnyx/telnyx-webrtc-android/blob/main/telnyx_rtc/build.gradle)
- Login is by SIP credentials (`credentialLogin`) or JWT (`tokenLogin`). Events arrive on a Kotlin `SharedFlow` (`socketResponseFlow`), and the LiveData API is deprecated. Calls are held in a UUID-keyed map, so several calls can run at once. Trickle ICE and relay-only (`forceRelayCandidate`) are supported. — [README](https://github.com/team-telnyx/telnyx-webrtc-android)
- Push works like this. The app passes an FCM token in `CredentialConfig`/`TokenConfig`, and a push credential is configured in the Telnyx Portal. The Telnyx backend sends the FCM push, and the app then reconnects the socket and "attaches" to the call (`telnyx_rtc.attachCalls`). `connectWithDeclinePush()` declines from the push without opening the full app. "Missed call!" and "Answered Elsewhere" cleanup pushes exist. The foreground service uses type `phoneCall`. — [README](https://github.com/team-telnyx/telnyx-webrtc-android); [Telnyx push quickstart](https://developers.telnyx.com/docs/voice/webrtc/android-sdk/push-notification/quickstart)
- Telnyx also ships an MIT Flutter SDK (`team-telnyx/flutter-voice-sdk`) and a React Native "voice commons" layer (`team-telnyx/react-native-voice-commons`). The RN layer has no licence detected by GitHub. It offers CallKit/ConnectionService, FCM/APNs and an RxJS state machine. — [GitHub flutter-voice-sdk](https://github.com/team-telnyx/flutter-voice-sdk); [GitHub react-native-voice-commons](https://github.com/team-telnyx/react-native-voice-commons)

**Twilio Voice SDK**
- The mobile SDK signalling is a TLS connection over TCP 443 to `chunderm.gll.twilio.com` (and regional `chunderm.{region}.gll.twilio.com`). Registration goes to `ers.twilio.com:443`. Versions before 3.x used port 10194. Twilio's docs do not describe it as SIP or WebSocket. — [Twilio network connectivity requirements](https://www.twilio.com/docs/voice/sdks/network-connectivity-requirements)
- Since Voice SDK 3.0 the media engine has been built on Chromium's WebRTC. — [Twilio blog: Voice SDK 3.0 GA](https://www.twilio.com/en-us/blog/products/launches/voice-sdk-3-0)
- Licensing: `twilio/voice-quickstart-android` is MIT. `twilio/twilio-voice-react-native` is Apache-2.0 (its LICENSE file reads "Copyright (C) 2023-2026 Twilio, Inc. Licensed under the Apache License, Version 2.0"). The core Android/iOS Voice SDKs ship as binaries, and I found no source repo for them. — [GitHub voice-quickstart-android](https://github.com/twilio/voice-quickstart-android); [GitHub twilio-voice-react-native](https://github.com/twilio/twilio-voice-react-native)

**Vonage Client SDK**
- It is built on WebRTC, and the Android SDK is split into voice and chat modules with a minimum API of 23. Vonage's technical-details page says only that the SDK handles "call signaling". It names no protocol and no push provider, and says nothing about open source. — [Vonage technical details](https://developer.vonage.com/en/vonage-client-sdk/technical-details); [Vonage blog: new Client SDK](https://developer.vonage.com/blog/introducing-the-new-vonage-client-sdk-for-android-and-ios)
- In-App Voice can reach SIP-enabled devices and connect to other services over WebSockets on the server side. — [Vonage In-App Voice overview](https://developer.vonage.com/vonage-client-sdk/in-app-voice/overview)

**Plivo**
- Plivo no longer supports its native Android/iOS SDKs and points developers to PJSIP or Linphone. A changelog entry says the integrated WebRTC library was replaced with JsSIP. Plivo SIP endpoints register at `phone.plivo.com` on 5060 UDP/TCP and 5061 TLS. — [Plivo SDK FAQ](https://docs.plivo.com/docs/faq/voice-api-and-SIP-trunking/sdk-and-endpoints); [Plivo Android changelog](https://www.plivo.com/docs/sdk/client/android/changelog/)
- `plivo/plivo-android-sdk` on GitHub has no licence, 0 stars and a last push on 2025-11-26. — [GitHub plivo-android-sdk](https://github.com/plivo/plivo-android-sdk)

**Sinch**
- The Android In-App Calling SDK supports app-to-app, app-to-PSTN and app-to-SIP calling. I found no protocol or licence details. — [Sinch Android docs](https://developers.sinch.com/docs/in-app-calling/android-cloud)

**SignalWire (FreeSWITCH company)**
- I found no official native Kotlin/Swift SDK. Mobile support is the "RELAY Mobile SDK", described as React Native toolkits with WebRTC media, SIP bridging and PSTN. Call Fabric reaches mobile through a JS SDK with React Native adaptors. — [SignalWire RELAY Mobile SDK blog](https://signalwire.com/blog/relay-mobile-sdk-software-development-kit); [SignalWire open-source SDK reference app press release](https://www.businesswire.com/news/home/20250807936328/en)
- `signalwire/signalwire-react-native-unofficial` is MIT and was created 2026-08-07. Its README says: "Alpha. On its way to becoming SignalWire's supported React Native SDK". It wraps `@signalwire/js` v4 with react-native-webrtc, callkeep and react-native-incall-manager, and uses CallKit/ConnectionService. It needs RN 0.76+ with the New Architecture only. The README also says "No Android row is signed off": Android outbound audio and FCM+ConnectionService inbound are only "observed" on an emulator. — [GitHub signalwire-react-native-unofficial](https://github.com/signalwire/signalwire-react-native-unofficial)
- WebRTC.ventures built a native Android SDK for SignalWire's WebRTC infrastructure as a custom client project aiming at parity with the RN SDK. It is not a public SignalWire release. — [WebRTC.ventures blog](https://webrtc.ventures/2024/11/building-a-native-android-sdk-for-signalwires-webrtc-infrastructure/)
- `signalwire/signalwire-js` currently shows no licence and 3 stars on GitHub. The legacy SDKs `signalwire-legacy-js` and `signalwire-legacy-node` are MIT. — GitHub API, [signalwire org](https://github.com/signalwire)

**Wildix**
- The platform is built primarily on WebRTC. The mobile app disconnects from the SIP/presence server when backgrounded and relies on push. Media relays through `turn.wildix.com` on TCP 443. Wildix's roadmap mentions a mobile app "based on Reactive Native" [sic]. — [Wildix blog](https://blog.wildix.com/?p=85339); [Wildix Android app guide](https://wildix.atlassian.net/wiki/display/DOC/Android+Collaboration+Mobile+App+-+User+Guide)

**Acrobits / Groundwire**
- This is a commercial SIP Mobile SDK with a higher-level "Facade" over a raw SIP engine (libSoftphone). Push comes from Acrobits' SIPIS servers, which register the SIP account on the device's behalf. They "therefore need to know the user's SIP credentials". — [Acrobits SDK docs](https://doc.acrobits.net/sdk/index.html); [Acrobits FAQ](https://doc.acrobits.net/troubleshooting/faq/); [Acrobits SDK page](https://acrobits.net/acrobits-sdk/)

**Linphone**
- `linphone-android` is GPL-3.0 and `linphone-sdk` is AGPL-3.0 (GitHub metadata). Belledonne also sells proprietary licences, which I did not verify here. The stack supports SIP over UDP/TCP/TLS, SRTP/ZRTP/DTLS-SRTP and ICE/STUN/TURN. Push relies on the Flexisip proxy, so third-party SIP accounts do not get Linphone push automatically. — [GitHub linphone-android](https://github.com/BelledonneCommunications/linphone-android); [GitHub linphone-sdk](https://github.com/BelledonneCommunications/linphone-sdk); [Linphone SDK guide](https://wiki.linphone.org/xwiki/wiki/public/view/Lib/)

**Zoiper / 3CX**
- Zoiper: push is a paid feature. 3CX: no reliable source on its mobile signalling or push architecture. — [voip.ms softphones wiki](https://wiki.voip.ms/article/Softphones)

### Inferences
- The common design across vendors is a JSON or proprietary signalling channel over TLS or WSS on 443, libwebrtc for media, and a backend push service (FCM/APNs or PushKit) that wakes the app so it can reconnect and "attach" to the ringing call. Telnyx does this explicitly with `attachCalls`. SIP-native apps (Acrobits SIPIS, Linphone Flexisip) solve the same problem with a server-side registrar or push proxy that holds the registration.
- Telnyx's protocol is a fork of FreeSWITCH Verto (`verto.invite` became `telnyx_rtc.invite`). Its Kotlin socket code could serve as a starting point for a Verto client against FreeSWITCH `mod_verto`, but it is not SIP over WSS.

### Gaps
- Twilio's exact wire protocol on `chunderm` (it is often said to be a SIP variant over TLS, but Twilio's docs do not say). I did not find a primary source.
- Vonage and Sinch signalling protocols and SDK licence terms: their docs do not say.
- 3CX and Zoiper internals and push design: no primary sources found.
- Linphone's dual licensing terms: not checked on linphone.org in this pass.

## 2. Cross-platform native stacks: JS/Dart SIP UA plus native WebRTC

### Takeaway
The pattern "SIP-over-WSS UA in a managed language plus native libwebrtc bindings" is mature and permissively licensed end to end. In Flutter it is dart-sip-ua (MIT) with flutter-webrtc (MIT). In React Native it is JsSIP (MIT) or SIP.js (MIT) running in Hermes, with react-native-webrtc (MIT) and callkeep for CallKit/ConnectionService. PJSIP-based RN wrappers are GPL and look stale.

### Cited Findings
- `flutter-webrtc/dart-sip-ua` (pub.dev `sip_ua`) is MIT and "a dart-lang version of the SIP UA stack, ported from JsSIP". Transports are WebSocket and TCP, with no UDP signalling. Media goes through flutter-webrtc. It targets iOS, Android, Web, macOS, Windows, Linux and Fuchsia. It has about 374 stars, 507 commits and 163 open issues, and the GitHub Releases section is empty. — [GitHub dart-sip-ua](https://github.com/flutter-webrtc/dart-sip-ua)
- `flutter-webrtc/flutter-webrtc` is MIT, has about 4.5k stars, and was last pushed 2026-11-11. — GitHub API, [flutter-webrtc](https://github.com/flutter-webrtc/flutter-webrtc)
- `react-native-webrtc/react-native-webrtc` is MIT, has about 5.0k stars, and was last pushed 2026-11-14. — GitHub API, [react-native-webrtc](https://github.com/react-native-webrtc/react-native-webrtc)
- JsSIP is MIT (LICENSE.md: "Copyright (c) 2012-2015 José Luis Millán - Versatica ... The MIT License"). It has about 2.6k stars and was last pushed 2026-10-21. GitHub shows its SPDX as NOASSERTION only because of the file format. — [JsSIP LICENSE.md](https://github.com/versatica/JsSIP/blob/master/LICENSE.md)
- SIP.js (`onsip/SIP.js`) is MIT, has about 2.1k stars, and was last pushed 2026-06-15. — GitHub API, [SIP.js](https://github.com/onsip/SIP.js)
- SignalWire's alpha RN SDK shows the same composition in a vendor product: a JS SDK, react-native-webrtc, callkeep, react-native-incall-manager, and FCM+ConnectionService / PushKit+CallKit. — [GitHub signalwire-react-native-unofficial](https://github.com/signalwire/signalwire-react-native-unofficial)
- PJSIP is dual-licensed (GPLv2+ or commercial). The RN wrapper `react-native-pjsip` (npm `elburu-react-native-pjsip`) is GPL-3.0, at v1.0.0, with about 1 weekly download. — [PJSIP licensing](https://docs.pjsip.org/en/2.16/overview/license_pjsip.html); [npm elburu-react-native-pjsip](https://npmjs.com/package/elburu-react-native-pjsip)

### Inferences
- For a Kotlin team, these stacks show the layering to copy: SIP UA, then a WebRTC peer connection, then a call manager (Telecom/CallKit), then push. A Kotlin team cannot reuse them directly except by running JsSIP in an embedded JS engine, which these projects do not do on Android natively.
- dart-sip-ua is a direct port of JsSIP. Porting JsSIP's transaction and dialog logic to Kotlin follows the same precedent, and JsSIP's MIT licence allows it.

### Gaps
- Capacitor SIP or WebRTC plugins: not researched in this pass (I ran out of tool budget). Their licences and maturity are unknown.
- I did not verify a package literally named `react-native-sip`.
- I did not quantify dart-sip-ua's last release date or how active its maintainers are beyond the commit and issue counts.

## 3. How separable is Telnyx's `telnyx_common` and its samples from the Telnyx client?

### Takeaway
It is not separable as-is. `telnyx_common` declares `api project(':telnyx_rtc')`, and its core classes import Telnyx SDK types (`TelnyxClient`, `Call`, `PushMetaData`, and others). Because it is MIT, its structure can be lifted freely: use cases, a Room call history, an FCM service, notification and foreground services, and a core-telecom sample. The call-engine calls would need to go behind an interface. The `connection_service_app` sample's telecom package is the most loosely coupled part.

### Cited Findings
- Repo modules: `telnyx_rtc` (SDK), `telnyx_common`, and `samples/compose_app`, `samples/xml_app` and `samples/connection_service_app`. — [GitHub tree](https://github.com/team-telnyx/telnyx-webrtc-android)
- Contents of `telnyx_common`:
  - Top level: `TelnyxCommon.kt` (405 lines), `TelnyxViewModel.kt` (1681 lines) and `ProfileManager.kt`.
  - `data/`: a Room call history (`CallHistoryDao/Database/Entity/Repository`).
  - `domain/authentication/`: `AuthenticateBySIPCredentials`, `AuthenticateByToken`, `AuthenticateAnonymously` and `Disconnect`.
  - `domain/call/`: `SendInvite`, `AcceptCall`, `RejectCall`, `HoldUnholdCall`, `HoldCurrentAndAcceptIncoming`, `EndCurrentAndAcceptIncoming`, `EndCurrentAndUnholdLast` and `OnByeReceived`.
  - `domain/push/`: `AnswerIncomingPushCall`.
  - `notification/`: `MyFirebaseMessagingService`, `CallNotificationService`, `LegacyCallNotificationService`, `CallNotificationReceiver` and `BackgroundCallDeclineService`.
  - `service/`: `CallForegroundService`.
  — [GitHub tree](https://github.com/team-telnyx/telnyx-webrtc-android/tree/main/telnyx_common)
- Coupling evidence:
  - `telnyx_common/build.gradle` has `api project(':telnyx_rtc')` and `api 'com.google.firebase:firebase-messaging-ktx:24.1.0'`, plus Room, security-crypto and lifecycle.
  - `TelnyxCommon.kt` imports `com.telnyx.webrtc.sdk.TelnyxClient`, `Call`, `PushMetaData`, `AudioCodec`, `SocketConnectionMetrics`, `CallQualityMetrics` and `ICECandidate`.
  - `AcceptCall.kt` imports `sdk.Call` and `AudioConstraints`.
  - `CallForegroundService.kt` imports `sdk.model.PushMetaData`.
  — [telnyx_common/build.gradle](https://github.com/team-telnyx/telnyx-webrtc-android/blob/main/telnyx_common/build.gradle); [TelnyxCommon.kt](https://github.com/team-telnyx/telnyx-webrtc-android/blob/main/telnyx_common/src/main/java/com/telnyx/webrtc/common/TelnyxCommon.kt)
- The Telnyx docs describe `telnyx_common` as a bridge between the app and `telnyx_rtc`, with ready-made call UI and services, used by both the Compose and XML samples. — [Telnyx Android quickstart](https://developers.telnyx.com/docs/development/webrtc/android-sdk/quickstart)
- Despite its name, `samples/connection_service_app` uses Jetpack `androidx.core:core-telecom:1.0.0-beta01`. Its files are `TelecomCallService`, `TelecomCallManager`, `TelecomCallNotificationManager`, `TelecomCallActivity`, `TelecomCallScreen`, `TelecomCallBroadcast`, and `TelecomCall`/`TelecomCallAction`/`TelecomCallRepository` models, plus Hilt-style `di/TelecomModule` and `TelnyxClientModule`. It depends on both `telnyx_rtc` and `telnyx_common`. `TelecomCallService.kt` imports only its own telecom model classes from the `com.telnyx` namespace and no SDK types. — [connection_service_app tree](https://github.com/team-telnyx/telnyx-webrtc-android/tree/main/samples/connection_service_app); [build.gradle](https://github.com/team-telnyx/telnyx-webrtc-android/blob/main/samples/connection_service_app/build.gradle)

### Inferences
- The use-case and service structure (auth, invite, accept, hold or swap, push answer, foreground service, Room history, decline-from-push service) is engine-agnostic in shape. Telnyx's concrete types are woven through it, though. Reusing it on another engine (for example a Kotlin SIP-over-WSS UA plus libwebrtc) would take a fork: replace `TelnyxClient`/`Call`/`PushMetaData` with your own `CallEngine` interface and rewrite the roughly 1.7k-line ViewModel. The MIT licence permits this with attribution.
- The core-telecom sample (`utility/telecom/*`) is the easiest to transplant, since its service layer does not touch SDK types. It uses a beta core-telecom version, so check it against current core-telecom releases.
- The push flow (FCM push, reconnect, then `attachCalls`) is tied to Telnyx's backend. A FreeSWITCH or SIP deployment needs its own push gateway, such as a FreeSWITCH push module, a Kamailio/Flexisip-style proxy, or an app-server-driven FCM.

### Gaps
- I did not read the Compose sample UI code to judge how separable it is. I did not check the iOS repo's equivalent common layer.

## 4. SignalWire/FreeSWITCH-specific mobile SDKs: does any target SIP over WSS on FreeSWITCH directly?

### Takeaway
I found none. SignalWire's mobile offerings target SignalWire's cloud (RELAY or Call Fabric over its own protocol) through React Native, not FreeSWITCH SIP over WSS. For a self-hosted FreeSWITCH, the options are generic SIP-over-WSS UAs (JsSIP, SIP.js, dart-sip-ua) against `mod_sofia` WSS, or a Verto client against `mod_verto`, where Telnyx's MIT Kotlin code is the closest relative.

### Cited Findings
- SignalWire's RELAY Mobile SDK is React Native and aimed at SignalWire's platform. The Call Fabric mobile path is a JS SDK with RN adaptors. — [SignalWire blog](https://signalwire.com/blog/relay-mobile-sdk-software-development-kit); [press release](https://www.businesswire.com/news/home/20250807936328/en)
- The new alpha RN SDK wraps `@signalwire/js` v4 (SignalWire platform), is MIT, and Android is not signed off. — [GitHub signalwire-react-native-unofficial](https://github.com/signalwire/signalwire-react-native-unofficial)
- SignalWire's org on GitHub has many MIT server-side SDKs (Python, TypeScript, Go, Java, Rust, C, and others: "AI Agents ... Relay and REST"). These are not mobile calling clients. — GitHub API search of [github.com/signalwire](https://github.com/signalwire)
- dart-sip-ua's FAQ addresses FreeSWITCH/Asterisk-style server config problems such as missing DTLS/SRTP fingerprints and 488 codec mismatches. That suggests it is used against self-hosted PBX WSS. — [GitHub dart-sip-ua](https://github.com/flutter-webrtc/dart-sip-ua)

### Inferences
- "SignalWire SDK" does not mean "FreeSWITCH SDK". No vendor ships a supported native Kotlin SIP-over-WSS client for FreeSWITCH. A Kotlin team has three paths: (a) write or port a SIP-over-WSS UA (JsSIP/SIP.js are MIT port sources) on top of libwebrtc, (b) speak Verto to `mod_verto` by adapting Telnyx's MIT socket and JSON-RPC code, or (c) use a GPL or commercial native SIP stack (Linphone, PJSIP, Acrobits) over TLS rather than WSS.

### Gaps
- I did not confirm whether any community Kotlin or Java SIP-over-WSS library exists. That is out of this note's scope and likely covered by another researcher.
- FreeSWITCH-side mobile push support (for example a push module in `mod_sofia`) was not checked.
