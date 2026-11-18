# Incoming VoIP calls on mobile when the app is backgrounded or killed (SIP over WSS to FreeSWITCH), as of Nov 2026

Research date: 2026-11-18. Research was limited to about 17 tool calls. Apple developer pages render as JS and could not be fetched in full; Apple facts come from the docs JSON endpoint and Apple DTS forum replies.

## Android (API 34-36): ConnectionService vs Core-Telecom, FGS types, FCM priority, Doze, full-screen intent, battery exemptions

### Takeaway
On Android, the supported way to wake the app is an FCM high-priority (urgent) message. That message lets the app start a foreground service from the background. The app then registers the call with Telecom (Jetpack Core-Telecom `CallsManager`, which needs `MANAGE_OWN_CALLS` and a `phoneCall` FGS) and posts a CallStyle notification within 5 seconds. Google Play policy says calling apps that can use FCM may NOT request a battery-optimization exemption, so a permanent WebSocket kept alive by a foreground service is a policy and Doze risk. It is not the endorsed design.

### Cited Findings
**Core-Telecom / ConnectionService**
- The docs give the dependency `androidx.core:core-telecom:1.0.0`, i.e. a stable 1.0 line. The app declares `MANAGE_OWN_CALLS` and registers with `CallsManager(context).registerAppWithTelecom(capabilities)`. Incoming calls use `CallAttributesCompat(direction = DIRECTION_INCOMING)` and `callsManager.addCall(...)` with onAnswer/onDisconnect/onSetActive/onSetInactive lambdas. — [Android Dev: Core-Telecom](https://developer.android.com/develop/connectivity/telecom/voip-app/telecom)
- "The library uses `ConnectionService` on Android 13 (API level 33) and lower, or foreground service types on Android 14 (API level 34) and higher, for foreground support." — [Core-Telecom](https://developer.android.com/develop/connectivity/telecom/voip-app/telecom)
- To keep calls alive in the background, host `CallsManager` inside a foreground `Service` declared with `foregroundServiceType="phoneCall"`. The app must post a `NotificationCompat.CallStyle` notification and call `startForeground(..., FOREGROUND_SERVICE_TYPE_PHONE_CALL)` after adding the call. "You must post a call notification within 5 seconds of adding the call to the platform." — [Core-Telecom](https://developer.android.com/develop/connectivity/telecom/voip-app/telecom)
- Remote-surface callbacks (watch, BT headset, Android Auto) must complete within 5 s, or the call session may be torn down. The app must not use `AudioManager#setCommunicationDevice` or `startBluetoothSco` while using Telecom. It uses `requestEndpointChange()` and the `STREAM_VOICE_CALL` stream instead. — [Core-Telecom](https://developer.android.com/develop/connectivity/telecom/voip-app/telecom)
- Core call extensions (participants, local call silence) are stable. Raise-hand, kick and call icons are still `@ExperimentalAppActions`. — [Core-Telecom](https://developer.android.com/develop/connectivity/telecom/voip-app/telecom)

**Foreground service types**
- Since Android 14 (API 34), every FGS must declare a type plus a type-specific permission. — [FGS types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- `phoneCall`: permission `FOREGROUND_SERVICE_PHONE_CALL`. Prerequisite: the app declares `MANAGE_OWN_CALLS` OR holds `ROLE_DIALER`. Apps targeting Android 15+ cannot launch a phoneCall FGS from a `BOOT_COMPLETED` receiver. — [FGS types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- `microphone`: permission `FOREGROUND_SERVICE_MICROPHONE`, plus the runtime `RECORD_AUDIO` permission. `RECORD_AUDIO` is a while-in-use permission: "you cannot create a `microphone` foreground service while your app is in the background", with a few exceptions. — [FGS types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- The while-in-use exemptions include: "The service starts by interacting with a notification", "A system component starts the service", and a PendingIntent from a visible app. — [FGS background-start restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)
- Since Android 12, apps cannot start an FGS from the background except in listed cases. The cases include: "Your app receives a high priority message using Firebase Cloud Messaging", the user acting on a notification, the user having turned off battery optimizations, and SYSTEM_ALERT_WINDOW (on Android 15+ only with a visible overlay). — [FGS background-start restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)
- If FCM downgrades the message to normal priority, starting an FGS throws `ForegroundServiceStartNotAllowedException`. Google recommends checking `RemoteMessage.getPriority() == PRIORITY_HIGH` before starting the service. — [FGS background-start restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)
- Apps targeting Android 14+ must declare their FGS types in Play Console (Policy > App content). — [FGS types](https://developer.android.com/develop/background-work/services/fgs/service-types)

**FCM priority and deprioritization**
- FCM may downgrade high-priority messages to normal, or proxy them through Play services, when it sees a pattern of messages that don't produce user-facing notifications. FCM evaluates 7 days of behaviour per app instance. Showing visible notifications prevents future deprioritization. If the user has disabled notifications, messages get deprioritized. — [FCM message priority](https://firebase.google.com/docs/cloud-messaging/android/message-priority)
- High-priority messages are delivered immediately and can wake a dozing device, with "very limited network access". `onMessageReceived` gets "several seconds", slightly more for high priority. Google advises against calling your server before showing the notification. — [FCM message priority](https://firebase.google.com/docs/cloud-messaging/android/message-priority)
- HTTP v1 API messages are notification-proxied by default on Android Q+ with recent Play services. Opt-outs: a manifest meta-data entry, `setNotificationDelegationEnabled`, or `proxy: DENY` per message. Proxying is labelled Beta. — [FCM message priority](https://firebase.google.com/docs/cloud-messaging/android/message-priority)
- NEW (preview): FCM "v2" Android config (`AndroidConfigV2`, Pre-GA, SDK `firebase-messaging:26.1.0-eap01`) replaces explicit priority with message types. `RemoteNotification` is urgent and user-visible, "delivered without delay and can bypass battery-saving measures". `BackgroundSync` is non-urgent and can be batched or delayed. Devices fall back to `onMessageReceived()` when the v2 callbacks aren't implemented. — [FCM v2 API](https://firebase.google.com/docs/cloud-messaging/send/v2-api)
- The Android Doze page already says: "v1 of FCM used high priority messages whereas v2 uses `RemoteNotification` ... We strongly recommend migrating to v2 as v1 is deprecated." — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby). This conflicts with the FCM v2 page, which still calls v2 a Pre-GA preview and gives no v1 deprecation timeline. — [FCM v2 API](https://firebase.google.com/docs/cloud-messaging/send/v2-api)

**Doze / App Standby and WebSockets**
- In Doze, the system "Suspends network access", ignores wake locks, defers alarms, jobs and WorkManager, and opens periodic maintenance windows that become less frequent over time. — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)
- `setExactAndAllowWhileIdle()` can fire at most once per 9 minutes per app. — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)
- App Standby does not apply while the app has a foreground process (an activity or FGS). But: "Don't start a foreground service just to prevent the system from determining that your app is idle." Idle apps get network roughly once a day. — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)
- "If your app requires a persistent connection to the network to receive messages, use FCM if possible." — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)

**Battery optimization exemption policy**
- Google's exemption table:
  - "Instant messaging, chat, or calling app ... Yes, using FCM → Not Acceptable".
  - Not acceptable either if the app can use FCM but doesn't use `RemoteNotification`.
  - Acceptable only if the app "can't use FCM because of technical dependency on another messaging service or Doze and App Standby break the core function"; "enterprise VOIP apps" are named in that row.
  - Google Play prohibits requesting direct exemption unless the app's core function is adversely affected.
  - Users can still exempt the app manually via `ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS`.
  — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)

**Full-screen intent**
- From January 22, 2025, for apps targeting Android 14+, `USE_FULL_SCREEN_INTENT` is granted by default only to apps with calling or alarm functionality. Other apps need user approval via `ACTION_MANAGE_APP_USE_FULL_SCREEN_INTENT`. A Play Console declaration has been required since May 31, 2024. — [Play: FGS & FSI requirements](https://support.google.com/googleplay/android-developer/answer/13392821); [Play policy preview](https://support.google.com/googleplay/android-developer/answer/16965181)
- The Play Store revokes the FSI permission at install for apps without calling or alarm functionality. — [AOSP FSI limits](https://source.android.com/docs/core/permissions/fsi-limits)
- An April 2026 Play policy preview exists. My search found no FSI-specific change in it. — [Play policy preview](https://support.google.com/googleplay/android-developer/answer/16965181)

### Inferences
- Proven Android incoming-call path for a killed app:
  1. FCM high-priority data message, or v2 `RemoteNotification`.
  2. `onMessageReceived` checks `priority == PRIORITY_HIGH`.
  3. Start a `phoneCall` FGS (allowed: FCM-high exemption plus `MANAGE_OWN_CALLS`).
  4. Call `CallsManager.addCall(INCOMING)`.
  5. Post a CallStyle notification with a full-screen intent within 5 s.
  6. In parallel, open the WSS connection and send REGISTER, then receive the parked INVITE.
- Microphone capture after the user answers. Answering from the CallStyle notification counts as "interacting with a notification", which is a while-in-use exemption, so mic access should work. Declaring `phoneCall|microphone` on the service and upgrading the type after answer is common practice. I did not find a Google page describing this exact combination, so it is inference.
- The app must post a visible call notification for every call push, including calls that are cancelled before the app wakes (show a missed-call notification). Otherwise FCM's 7-day heuristic deprioritizes the app and the FGS start starts failing.
- The phoneCall FGS should only run during a call. Using it as a permanent "always registered" keep-alive goes against its documented purpose ("Continue an ongoing call") and against the Doze guidance.
- A self-hosted PBX app that targets enterprise customers could argue the "enterprise VOIP" row for an exemption. A consumer Play app that could use FCM cannot.

### Gaps
- Whether a running FGS exempts the app's sockets from Doze network suspension: I did not fetch the power-details page. Developer reports differ, and OEM battery killers (Xiaomi, Samsung and others) add more variance. Not verified.
- The latest Core-Telecom version beyond 1.0.0 (for example 1.1.x betas) and its minSdk were not verified.
- Android 16 (API 36) changes specific to calling apps were not separately researched.

## iOS: PushKit, CallKit, background WebSocket, audio session, APNs

### Takeaway
On iOS, a PushKit VoIP push is the only reliable wake path for a background or killed SIP app. Every VoIP push must be reported to CallKit (`reportNewIncomingCall`) before the delegate returns. If not, iOS 13+ kills the app, and repeated failures stop VoIP push delivery to the app; developers report that only a reinstall restored it. A persistent background WebSocket is not available to the app.

### Cited Findings
- "For apps built using the iOS 13 SDK or later, PushKit requires you to use CallKit when handling VoIP calls." From the push payload, create a `CXCallUpdate` and call `CXProvider.reportNewIncomingCall(with:update:completion:)`. "While CallKit processes your request, establish a connection to your VoIP server in parallel." If the connection fails, call `reportCall(with:endedAt:reason: .failed)`. — [Apple: Responding to VoIP Notifications from PushKit](https://developer.apple.com/documentation/pushkit/responding-to-voip-notifications-from-pushkit) (fetched via the docs JSON endpoint)
- "Set the value of the `apns-expiration` header field to `0`, or to only a few seconds", so late calls are not delivered. — [Apple: Responding to VoIP Notifications](https://developer.apple.com/documentation/pushkit/responding-to-voip-notifications-from-pushkit)
- "On iOS 13.0 and later, if you fail to report a call to CallKit, the system will terminate your app. Repeatedly failing to report calls may cause the system to stop delivering any more VoIP push notifications to your app." — Apple DTS, cited in [Apple Dev Forums thread 764376](https://developer.apple.com/forums/thread/764376); same rule in the [PushKit delegate reference](https://developer.apple.com/tutorials/data/documentation/pushkit/pkpushregistrydelegate/pushregistry(_:didreceiveincomingpushwith:for:completion:).md)
- Developers report that delivery resumed only after an uninstall and reinstall. There is no official API to detect the block. — [Forums 764376](https://developer.apple.com/forums/thread/764376); [Forums 768136](https://developer.apple.com/forums/thread/768136)
- Slow debug builds can miss the deadline, and the app gets killed (crash code 0xBAADCA11 is linked to VoIP push handling). — [Forums 781211](https://developer.apple.com/forums/thread/781211)
- Linphone's iOS docs describe their PushKit and CallKit handling for a SIP stack. — [Linphone wiki: iOS push notification management](https://wiki.linphone.org/xwiki/wiki/public/view/Lib/Features/IOS%20push%20notification%20management/)

### Inferences (not verified against a fetched Apple page in this session)
- APNs VoIP push needs these headers:
  - `apns-push-type: voip`
  - `apns-topic: <bundle-id>.voip`
  - priority 10
  - token-based (.p8) or VoIP-services certificate auth over HTTP/2
  The fetched Apple text did not include these details. This is standard knowledge and should be checked against Apple's "Sending notification requests to APNs" page.
- Audio session: with CallKit, configure `AVAudioSession` (category `.playAndRecord`, mode `.voiceChat`), but activate audio and the WebRTC/RTP stack only in `provider(_:didActivate:)`. Deactivate in `didDeactivate`. This is the well-known CallKit pattern and was not fetched here.
- A background WebSocket gets suspended within seconds of backgrounding. The old `voip` keep-alive socket (`setKeepAliveTimeout`) was deprecated in iOS 10, so SIP-over-WSS clients must re-connect and re-REGISTER after each push. Not verified in this session.
- Because the push must carry caller info for CallKit before SIP connects, the push sender (the gateway or FreeSWITCH hook) should include the caller ID and a call-id/UUID in the payload. The app later matches the parked INVITE to that UUID, or ends the CallKit call if the INVITE never arrives.

### Gaps
- The Apple pages render as JS, so I could not get verbatim APNs header requirements or the newer iOS 17/18/26 CallKit/LiveCommunicationKit changes. I don't know whether any 2026 iOS release changed PushKit rules.
- The `com.apple.developer.pushkit.unrestricted-voip` entitlement (for exemptions such as some enterprise/PTT cases) was not confirmed.

## SIP side: RFC 8599, FreeSWITCH push options, push gateways, WSS clients that sleep

### Takeaway
RFC 8599 defines the standard design:
1. The UA puts `pn-provider`, `pn-param` and `pn-prid` in the REGISTER Contact.
2. On an incoming request, the proxy sends a push and parks the request.
3. The woken UA re-REGISTERs.
4. The proxy forwards the parked INVITE to the new binding.

FreeSWITCH has no native RFC 8599 support. Deployments either put an RFC 8599 proxy in front of FreeSWITCH (OpenSIPS `mid_registrar`, Flexisip Push Gateway, Kamailio with tsilo plus scripting), or script it inside FreeSWITCH (sofia::register event plus Lua/ESL that pushes and waits for re-register before bridging). The second option is the smallest for a team that already owns FreeSWITCH and Lua routing.

### Cited Findings
**RFC 8599**
- URI parameters:
  - `pn-provider`: registered values are `apns`, `fcm`, `webpush`.
  - `pn-param`: APNs Team ID + Topic, or the FCM Project ID.
  - `pn-prid`: the device token or FCM registration token. Omitting it means stop pushing.
  - `pn-purr`: a proxy-issued reference for pushes during a dialog.
  — [RFC 8599](https://www.rfc-editor.org/rfc/rfc8599.html)
- The proxy puts the request in a "Request Push Bucket" with a timer and sends the push. It waits for the 2xx to the UA's REGISTER, checking that the Contact matches, then forwards the request. On failure or timeout it returns an error (480 on timeout, 404/480 recommended). In testing, the REGISTER arrived about 2 s after the push request. — [RFC 8599](https://www.rfc-editor.org/rfc/rfc8599.html)
- On a push, the UA must send a binding-refresh REGISTER with the same pn-* parameters and may discard the payload. 555 "Push Notification Service Not Supported" is a response to REGISTER only. `sip.pnsreg` signals that the UA can refresh without push; the proxy value should be above 120 s. The proxy should push at least 120 s before a binding expires. — [RFC 8599](https://www.rfc-editor.org/rfc/rfc8599.html)

**FreeSWITCH**
- FreeSWITCH has no native RFC 8599 registrar. — [SessionTalk blog](https://sessiontalk.io/blog/sip-push-notifications-asterisk-freeswitch) (vendor blog that promotes its own cloud softphone; its technical description matches the community module READMEs)
- The community `mod_apn` (sem32/freeswitch-PushNotificator, devopsec fork) works like this:
  - It listens to `sofia::register` events and stores tokens from the Contact.
  - When a call targets the device, it sends a push.
  - It waits for a new REGISTER, then originates or bridges the call.
  — [sem32/freeswitch-PushNotificator](https://github.com/sem32/freeswitch-PushNotificator); [devopsec fork](https://github.com/devopsec/freeswitch-ApplePushNotification)
- SessionTalk says mod_apn was built on Apple's legacy binary APNs protocol, has inconsistent HTTP/2 updates, no native FCM support, and is updated only sporadically. — [SessionTalk blog](https://sessiontalk.io/blog/sip-push-notifications-asterisk-freeswitch)
- A PBXForums (FusionPBX) community Lua approach reads `pn-provider` and `pn-prid` from `sofia_contact` output and sends a push before ringing. — [PBXForums: Push notifications for Ring Groups with Linphone](https://www.pbxforums.com/threads/push-notifications-for-ring-groups-with-linphone.9061/) (seen in a search summary only; not fetched)
- Caveats from the same sources: the Sofia profile must keep the full Contact URI with its parameters, and a woken device re-registers from a new IP/port. — [SessionTalk blog](https://sessiontalk.io/blog/sip-push-notifications-asterisk-freeswitch)

**Push gateways and proxies**
- **Flexisip** (Belledonne/Linphone):
  - Dual-licensed: AGPLv3 or a proprietary commercial licence; pricing is not published.
  - Supports RFC 8599, APNs and FCM.
  - Has a "Push Gateway" mode in front of an existing SIP server.
  — [Linphone: Flexisip SIP server](https://www.linphone.org/en/flexisip-sip-server/); [Flexisip supported standards](https://wiki.linphone.org/xwiki/wiki/public/view/Flexisip/Core%20servers/Features/Supported%20features%20and%20standards/)
- **Flexisip Push Gateway mode**:
  - The UA uses Flexisip as outbound proxy. Flexisip masquerades the Contact with a `CtRt<key>` parameter and relays REGISTER to the backend, which does authentication and must keep Contact parameters intact and should support Path (RFC 3327).
  - For an incoming INVITE, Flexisip first tries the UA directly, then pushes after a short delay. It re-sends the INVITE once the UA re-registers (`fork-late=true`).
  - Main config keys: `module::Registrar reg-on-response`, `module::Router fork-late`, `module::PushNotification`, `module::ContactRouteInserter`.
  - The page does not mention WebSocket transport.
  — [Flexisip wiki: Push Gateway](https://wiki.linphone.org/xwiki/wiki/public/view/Flexisip/Push%20Gateway/)
- **aria-push-gateway** (5060 Solutions):
  - Rust (Axum REST, SQLite device registry, SIP proxy manager, push manager, call handoff manager); depends on a sibling `aria-sip-core` crate.
  - It keeps SIP registrations on behalf of the mobile app. On an incoming call it sends APNs VoIP or FCM data push and answers 180 Ringing while the app wakes. When the app accepts via the REST API (e.g. `/v1/calls/{token}`), it answers 200 OK with the app's SDP.
  - Its diagram shows FreeSWITCH/Asterisk as the PBX.
  - Licence: README says "Proprietary — 5060 Solutions", although the repo is public. Maturity: 52 commits, 2 stars, no releases. No RFC 8599 or WebSocket mention.
  — [aria-push-gateway repo](https://github.com/5060-Solutions/aria-push-gateway)
- **OpenSIPS** `mid_registrar`:
  - RFC 8599 support since 3.1 LTS (`pn_enable`, optional `pn_enable_purr`).
  - Raises `E_UL_CONTACT_REFRESH` when a push is needed. `mid_registrar_lookup()` returns 2 when only PN contacts needing a push were found.
  - `event_routing` is required to match the REGISTER to the suspended INVITE.
  — [OpenSIPS mid_registrar 3.6](https://opensips.org/docs/modules/3.6.x/mid_registrar.html); [OpenSIPS blog Part II](https://blog.opensips.org/2020/06/03/sip-push-notification-with-opensips-3-1-lts-rfc-8599-supportpart-ii/)
- In January 2026, an OpenSIPS 3.4.16 + FCM user reported `mid_registrar_lookup` returning 1 and no refresh event, with no resolution found. A 2025 user reported the older `event_routing` style working well. — [OpenSIPS-Users Jan 2026](https://opensips.org/pipermail/users/2026-January/049182.html); [OpenSIPS-Users Jan 2025](https://opensips.org/pipermail/users/2025-January/048665.html)
- **Kamailio** `tsilo`: stores INVITE transactions keyed by R-URI (`ts_store`). On a later REGISTER, `ts_append`/`ts_append_by_contact` adds branches to the stored transaction. This is the building block for "park INVITE until wake-up REGISTER". The module docs do not mention push itself; that has to be scripted (e.g. with http_client to APNs/FCM). — [Kamailio tsilo](https://www.kamailio.org/docs/modules/stable/modules/tsilo.html)
- Commercial options named by SessionTalk: MizuVoIP MPUSH, 2600Hz Pusher (Kazoo), and cloud softphone vendors' own push relays. — [SessionTalk blog](https://sessiontalk.io/blog/sip-push-notifications-asterisk-freeswitch)
- AudioCodes SBCs have a configurable Push Notification Service. — [AudioCodes SBC manual](https://techdocs.audiocodes.com/session-border-controller-sbc/mediant-software-sbc/user-manual/version-740/content/um/Configuring%20Push%20Notification%20Service.htm) (search result only)

### Inferences
- **WSS-only clients.** When the app sleeps, its WebSocket (and with it the SIP flow) dies. The registration in `sofia` then points to a dead WS connection; with mod_sofia's WS transport, the contact is unreachable even before it expires. The workable designs:
  - **(a) Register with push params, then wake-then-register:** the UA sends `pn-provider`/`pn-param`/`pn-prid` as Contact URI params on the WSS REGISTER, with a long expiry (e.g. 1-7 days). The server must not depend on the WS being alive. On INVITE it pushes, parks, waits for the fresh REGISTER, then bridges to the new contact. This is RFC 8599 and the most common design.
  - **(b) Server-side registration proxy (aria-style):** the gateway stays registered and the app answers via push plus REST. The app does not need to re-register before the call.
  - **(c) Token stored out-of-band:** the app posts its FCM/APNs token to the platform's own REST API instead of the SIP Contact. FreeSWITCH/Lua looks up the token by user and pushes. This avoids depending on Contact-param preservation.
- **FreeSWITCH minimal recipe (inference from the mod_apn design and RFC 8599 semantics; not a documented upstream feature):**
  1. Store the push token, from the Contact params via the `sofia::register` event or from your own API.
  2. In the Lua dialplan for a user with no live WS registration, or always for mobile users, send the push via HTTP to FCM v1 / APNs.
  3. Send 180 Ringing and early ringback to the caller.
  4. Wait up to about 20-30 s for a fresh `sofia::register` for that user. Poll `sofia_contact`, or subscribe via ESL.
  5. Bridge to `sofia_contact(user@domain)`.
  6. When the caller hangs up first: on Android send a data push (or a missed-call notification) so the phone stops ringing; on iOS cancel over the connection the app opened after the VoIP push, not with another VoIP push (correction from the Codex docs review, per Apple's PushKit documentation).
- On iOS, Apple's rule that every VoIP push must produce a CallKit call means "registration-refresh pushes" (RFC 8599 binding refresh) cannot be sent as VoIP pushes. Use long registration expiries, and refresh when the app is foregrounded. (Inference, consistent with the DTS rule above.)
- Licensing for a small team:
  - Flexisip AGPL is acceptable if run unmodified as a separate network service. AGPL obligations apply mainly to modifications offered over a network; confirm with counsel.
  - aria-push-gateway is proprietary, so it cannot be used without a licence despite being public.
  - OpenSIPS (GPLv2) and Kamailio (GPLv2) add a whole proxy tier to operate.

### Gaps
- No FreeSWITCH official docs or GitHub issue confirming that mod_sofia preserves `pn-*` Contact params on WS registrations; it needs a lab test.
- No current (2025-2026) maintained FreeSWITCH push module was found. The mod_apn forks' last-commit dates were not checked.
- Flexisip Push Gateway's WebSocket support for client-side transport was not confirmed.
- I did not fetch the 5060 Solutions licence terms or pricing.

## Minimum viable approach for a first release: foreground-service keep-alive vs push

### Takeaway
A foreground-service keep-alive (a permanent WSS registration on Android) is the fastest thing to build, and it works for internal or enterprise users who whitelist the app. It doesn't work on iOS at all, it conflicts with Google's guidance and Play policy for calling apps that could use FCM, and it is unreliable under Doze and OEM battery management. The minimum design that is viable on both platforms and defensible on Play is push-based:
- **Client:** FCM high-priority data message (Android) or PushKit VoIP (iOS) → native call UI (Core-Telecom / CallKit) → connect WSS + REGISTER → receive the parked INVITE.
- **Server:** a small FreeSWITCH Lua + token store + FCM/APNs sender, with no extra proxy tier.

### Cited Findings
- Google: calling apps that can use FCM may not request a battery-optimization exemption; persistent-connection apps should use FCM. — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)
- Google: "Don't start a foreground service just to prevent the system from determining that your app is idle." The phoneCall FGS is described as for "an ongoing call". — [Doze & App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby); [FGS types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- An FCM high-priority message is an explicit exemption allowing an FGS start from the background. — [FGS restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)
- iOS requires PushKit plus CallKit for VoIP wake-ups. — [Apple: Responding to VoIP Notifications](https://developer.apple.com/documentation/pushkit/responding-to-voip-notifications-from-pushkit)
- RFC 8599 measured about a 2 s push-to-REGISTER delay. — [RFC 8599](https://www.rfc-editor.org/rfc/rfc8599.html)

### Inferences
- **Suggested phasing for a small team (Android-first KMP app):**
  - **Phase 0 (dev/internal):** a WSS registration while the app is in the foreground only, plus a Core-Telecom call flow. Optionally, a user-enabled "always online" FGS mode for enterprise users who exempt the app from battery optimization manually; document it as best-effort.
  - **Phase 1 (first public release):**
    - The app sends its FCM token at REGISTER, as `pn-provider=fcm;pn-param=<project>;pn-prid=<token>` Contact params and/or via the platform REST API.
    - FreeSWITCH Lua on an incoming call to a mobile user: send an FCM v1 data message with priority high and a short TTL (about 30 s), containing call-id, caller name/number and the server's WSS URL. Then wait up to about 25 s for a fresh registration and bridge. On caller hangup, send a cancel push.
    - App: in `onMessageReceived`, start the phoneCall FGS → `CallsManager.addCall(INCOMING)` → CallStyle + full-screen intent → connect WSS and REGISTER in parallel → match the INVITE by caller or a custom header → answer.
  - **Phase 2 (iOS):** the same server flow with an APNs VoIP push. CallKit report first, then connect.
  - **Later:** if scaling or multiple PBXs require it, move push into an RFC 8599 proxy (OpenSIPS `mid_registrar` or Flexisip Push Gateway) instead of FreeSWITCH Lua.
- **Trade-offs:**
  - Push adds about 2-5 s to call setup (wake + TLS/WSS + REGISTER + INVITE). FCM/APNs delivery is not guaranteed, so it needs a missed-call fallback.
  - It needs Firebase and Apple developer credentials on the server.
  - In exchange, battery use is near zero, it complies with Play policy, and it is the only option on iOS.
  - Keep-alive gives instant calls without server changes, but uses battery, faces Doze network suspension and OEM process killing, and carries Play review risk.

### Gaps
- No measured 2026 field data on FCM high-priority delivery latency or reliability per OEM was found.
- How existing Kotlin/KMP SIP stacks (e.g. Linphone SDK's built-in push handling) wire Core-Telecom versus raw ConnectionService was not researched here.
