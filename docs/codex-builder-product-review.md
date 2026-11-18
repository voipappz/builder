1. **Critical — Release 1 builds the wrong abstraction first.**  
   **Evidence:** Builder currently has one workspace, one agent working directory, and one optional app process ([launcher.rs](core/src/launcher.rs:355)); there is no platform/session model. The phone already works as a single native `kind = "app"` project with test/APK/install actions ([builder.toml](phone/builder.toml:1)).  
   **Inference:** A Web/Android picker would require multi-project lifecycle, configuration, logs, and agent-root semantics before delivering Android visibility. Make it a later convenience, not release 1.

2. **High — The smallest useful release is Android-only, one repo, one device.**  
   **Evidence:** Existing machinery already runs `make dev`, streams one log, and exposes declared actions; native apps count as healthy while their process runs ([bot.rs](core/src/bot.rs:316)).  
   **Inference:** Release 1 should open `phone/`, run its existing test loop, provide Build/Install actions, detect one ADB device, launch the app, and open scrcpy—initially in its own window. Success: a chat edit can be tested, installed, launched, and observed without a terminal. No Web, picker, credentials, calls, or embedded mirroring.

3. **High — “scrcpy in the pane” is presented as straightforward but is a separate technical risk.**  
   **Evidence:** Builder’s current preview is an HTTP page; a native app has no page URL. Nothing shown provides native-window capture or input forwarding.  
   **Inference:** Embedding scrcpy inside WebView/Tauri is OS-specific window parenting or video-stream integration, not normal framing. On Windows+WSL, ADB/USB ownership is another boundary: USB may require usbipd passthrough, while running the emulator and scrcpy on Windows may be simpler than KVM and GUI forwarding inside WSL. Add a proof release: external scrcpy first, embedded only after a spike.

4. **High — Release 3 mixes Builder work with unfinished phone capability.**  
   **Evidence:** `CLAUDE.md` says registration has only reached the 401 challenge and calls are next. Builder cannot demonstrate echo audio until the phone independently completes SIP authentication, dialogs, WebRTC audio, and device routing.  
   **Inference:** Define a prerequisite: “the same APK completes the call outside Builder.” Builder should then only install, launch, and expose logs. Do not make Builder own SIP credentials yet; the phone’s secure storage/login flow should.

5. **High — iOS is not merely Android through a Mac.**  
   **Evidence:** The plan leaves “streamed or screenshots” and Mac ownership undecided.  
   **Inference:** Remote Mac execution needs repository synchronization, command transport, signing/Xcode state, simulator control, log transport, display/input streaming, and secret boundaries. A rented Mac may prohibit low-latency interactive streaming. First iOS scope should be build/test/log on a named Mac; simulator viewing can remain on that Mac. Interactive remote embedding is a later experiment.

6. **Medium — Multi-repo projects are underspecified and unnecessary for personal-use v1.**  
   **Evidence:** The agent is deliberately confined to one workspace, while Web and phone live in different repositories.  
   **Inference:** A combined project weakens that safety boundary and raises unresolved questions about branches, worktrees, setup, concurrent processes, and which repository receives an edit. Keep one Builder project per repo. Later add a “workspace collection” that switches projects without granting one agent both roots by default.

7. **Medium — Release ordering should follow technical risk.**  
   **Inference:** Order: (1) Android external-device loop; (2) reliable rebuild/install/relaunch and logs; (3) optional embedded scrcpy; (4) project switching/platform picker; (5) call convenience after phone calls work independently; (6) Mac build/test transport; (7) iOS viewing; desktop last. The picker’s top-versus-side placement is premature; decide only after there are two proven platform adapters.
