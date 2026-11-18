**Verdict (inference): viable prototype, but the plan understates SIP and mobile lifecycle work.** Builder is fine as development tooling. I would approve a bounded interoperability spike before committing to the proposed code reuse.

Read-only throughout: no file changes, `.env` reads or switch connections.

1. **Your SIP core — promising experiment, substantial work remaining.**  
   **Evidence:** existing reports contain 17 passing JVM tests; I did not rerun them. The [build targets JVM only](/home/nir/voipappz/builder-native/phone/sip/build.gradle.kts:5). Registration handles bounded authentication retries and 423 responses, but automatic renewal, reconnect, incoming requests and dialogs remain absent. **Inference:** production reliability will depend primarily on transaction/dialog state machines, beyond message construction and digest.

2. **UVP — source audit remains unresolved.**  
   **Evidence:** GitHub source retrieval failed; I could read its [README](https://github.com/Frank-Coding97/uvp-gb28181-sim), which documents UDP/TCP, camera RTP and MD5 authentication. I therefore cannot honestly rate its dialog/transaction implementation or identify code-level defects. **Inference:** digest/parser utilities might transfer after inspection; its camera SDP should be replaced by libwebrtc-generated SDP. Do not budget its advertised INVITE/BYE/CANCEL support as a reusable telephone stack without that audit.

3. **Telnyx — useful reference, substantial adaptation.**  
   **Evidence:** inspected local source binds `TelnyxCommon`, notifications and foreground service to `TelnyxClient`, `Call` and `PushMetaData`. The [Telecom repository](https://github.com/team-telnyx/telnyx-webrtc-android/blob/0bbfe61faf95a246f2f8872ac84fe6eb8f470e1b/samples/connection_service_app/src/main/java/com/telnyx/webrtc/sdk/utility/telecom/model/TelecomCallRepository.kt) also consumes Verto socket events. **Inference:** extract selected patterns behind your own call-engine interface. Starting from Android’s smaller [Core-Telecom samples](https://developer.android.com/develop/connectivity/telecom/voip-app/telecom) may cost less than transplanting both Telnyx layers.

4. **webrtc-kmp — reasonable spike choice; interoperability likely, unproven.**  
   **Evidence:** it wraps native libwebrtc, supports Android/iOS, uses Apache-2.0, and remains on [M125](https://github.com/shepeliev/webrtc-kmp), with [v0.125.11 marked prerelease](https://github.com/shepeliev/webrtc-kmp/releases). Budget native-library maintenance and early iOS linking/audio validation.  
   **Inference:** ICE, DTLS-SRTP, BUNDLE, rtcp-mux and Opus should negotiate with a suitably configured FreeSWITCH endpoint. **Evidence:** your browser code supplies no SDP modifiers; [SIP.js 0.21.2](https://github.com/onsip/SIP.js/blob/0.21.2/src/platform/web/session-description-handler/session-description-handler.ts) applies none by default. Replicate its sequencing: set local description, await ICE gathering with a deadline, then send the updated `localDescription`. Its default gathering timeout is five seconds. No blanket FreeSWITCH SDP munging is established here.

5. **Biggest missing production requirements.**  
   **Inference:** prioritize inbound wakeup infrastructure, TURN including restrictive-network relay tests, renewal/keepalive/reconnect, and Wi-Fi–cellular transitions. SIP needs routing/tags/CSeq, authenticated INVITE retry and ACK, CANCEL/200 races, duplicate responses, re-INVITE/hold, session timers, early media and DTMF. Start without trickle; enable it only after confirming [SIP trickle support](https://www.rfc-editor.org/rfc/rfc8840.html). Background incoming calls need a server-side push/wakeup strategy; [RFC 8599](https://datatracker.ietf.org/doc/html/rfc8599) describes the SIP architecture. Plan FCM and iOS PushKit/CallKit integration.

6. **Cheapest proof sequence — your order is broadly right.**  
   **Recommendation:** offline protocol fixtures → live REGISTER/renew/unregister → bare physical-Android echo call with two-way audio and media statistics → inbound call, CANCEL/BYE, DTMF and hold → forced TURN/network change → minimal iOS call and background-push proof → app shell. Keep UI investment behind these gates.

7. **Simpler alternative worth testing.**  
   **Evidence:** [BSD baresip](https://github.com/baresip/baresip) supports Android/iOS; its [configuration supports WSS, ICE and DTLS-SRTP](https://github.com/baresip/baresip/wiki/Accounts). **Inference:** a shared Kotlin interface over JNI/C interop could materially reduce protocol work, with permissively licensed modules selected. Native bindings/builds remain work. I verified no turnkey pure-Kotlin alternative satisfying every constraint.