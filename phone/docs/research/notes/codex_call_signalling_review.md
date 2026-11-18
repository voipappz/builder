1. **Critical — inbound cancellation is unimplemented.** [Call.kt:243](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:243)  
   **Evidence:** `IncomingCall` handles INVITE responses, ACK, and BYE only; no CANCEL listener, `200 CANCEL`, or `487 INVITE`. The fake switch fabricates these itself ([CallTest.kt:60](phone/sip/src/jvmTest/kotlin/io/voipappz/sip/CallTest.kt:60)).  
   **Inference:** A FreeSWITCH caller cancelling while ringing will receive no CANCEL response, while the UI may continue ringing or answer a dead call. This is likely the first live signaling failure.

2. **Critical — messages are routinely lossy.** [SipClient.kt:31](phone/sip/src/commonMain/kotlin/io/voipappz/sip/SipClient.kt:31), [Call.kt:266](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:266), [Call.kt:277](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:277)  
   **Evidence:** `SharedFlow` has `replay=0`; buffering does not preserve events when there are no subscribers. Tests deliberately subscribe to BYE before answering ([CallTest.kt:101](phone/sip/src/jvmTest/kotlin/io/voipappz/sip/CallTest.kt:101)).  
   **Inference:** An immediate BYE, re-INVITE, OPTIONS, or NOTIFY arriving between per-operation collectors is dropped. Use one permanent dispatcher keyed by transaction/dialog, with buffered dialog mailboxes.

3. **High — UAC CANCEL lifecycle is racy/noncompliant.** [Call.kt:127](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:127)  
   **Evidence:** `cancel()` may run as soon as `invite` is assigned, before any provisional response—or even before `send()`. RFC 3261 requires waiting for a provisional response. There is no state synchronization with `start()`.  
   Correct pieces: same Request-URI, Via/branch, numeric CSeq, From/To, and Call-ID. It should also copy any original Route headers.

4. **High — dialog and UAS transaction handling is incomplete.** [Call.kt:181](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:181), [Call.kt:265](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:265)  
   **Evidence:** UAC correctly uses a new branch for 2xx ACK, the INVITE branch for non-2xx ACK, increments CSeq after authentication, reverses Record-Route, and uses Contact as remote target. However, strict routing (first Route without `lr`) is unsupported. UAS ACK matching checks only Call-ID; 200 INVITE is not retransmitted until ACK. BYE response copies only the top Via rather than every Via.  
   **Inference:** FreeSWITCH normally supplies loose Record-Route, so strict routing is less immediate; weak ACK matching and missing 200 retransmission remain correctness defects.

5. **High — real FreeSWITCH traffic is absent from tests.** [CallTest.kt:26](phone/sip/src/jvmTest/kotlin/io/voipappz/sip/CallTest.kt:26)  
   **Evidence:** The fake proxy has no Record-Route, 100 Trying, 183 SDP, re-INVITE, OPTIONS, NOTIFY, or session timers. `183` SDP is discarded ([Call.kt:210](phone/sip/src/commonMain/kotlin/io/voipappz/sip/Call.kt:210)).  
   **Inference:** Basic answered audio may connect, but early media will not; renegotiation and session refresh will fail, eventually causing media failure or FreeSWITCH BYE.

Before `webrtc-kmp`: introduce a permanent SIP transaction/dialog dispatcher and explicit call state machine; implement inbound CANCEL, re-INVITE and OPTIONS responses; preserve complete dialog headers/routes; expose provisional SDP; add cancellation cleanup and deterministic transaction/dialog timeouts; then test with realistic FreeSWITCH traces.