1. **High — CI does not validate the iOS targets.**  
   **Evidence:** [phone.yml](.github/workflows/phone.yml:18) runs only on Ubuntu, while `make test` executes only `:sip:jvmTest` and the APK covers only Android. Yet [shared/build.gradle.kts](phone/shared/build.gradle.kts:17) and [sip/build.gradle.kts](phone/sip/build.gradle.kts:15) declare three iOS targets. `kotlin.native.ignoreDisabledTargets=true` explicitly hides them on Linux.  
   **Inference:** iOS-specific source, Darwin dependencies, framework linkage, and `MainViewController` can regress unnoticed. Add a macOS job compiling/tests for simulator ARM64 before starting the Xcode app.

2. **Medium — the build toolchain is not reproducible.**  
   **Evidence:** [mise.toml](phone/mise.toml:5) specifies `android-sdk = "latest"`. Builder’s `make setup` and CI therefore install whichever SDK plugin/tool release is current, while Gradle expects platform/build-tools 35.  
   **Inference:** a future mise Android SDK change can break existing checkouts or make local and CI builds differ. Pin the mise tool version; ideally also enable dependency verification/locking for Gradle transitives.

3. **Medium — upcoming calling support lacks required platform permissions/configuration.**  
   **Evidence:** [AndroidManifest.xml](phone/androidApp/src/androidMain/AndroidManifest.xml:3) declares only `INTERNET`.  
   **Inference:** `webrtc-kmp` audio calls will require at least runtime `RECORD_AUDIO`; speaker/Bluetooth and background/incoming-call behavior will also need audio, Bluetooth, foreground-service, and notification permissions/services according to supported Android versions. iOS likewise needs microphone usage text and background/CallKit configuration, none of which exists yet. This does not break the current registration screen but blocks the stated next step.

4. **Low — probe mode can report success without receiving the promised digest challenge.**  
   **Evidence:** [RegisterCheck.kt](phone/sip/src/jvmMain/kotlin/io/voipappz/sip/RegisterCheck.kt:35) parses only `WWW-Authenticate`, but line 39 accepts either 401 or 407. A 407 normally uses `Proxy-Authenticate`; even a bare 401 succeeds.  
   Fix by selecting the header appropriate to the status and requiring a valid supported digest challenge.

5. **Low — remote `kind="app"` status is misleading.**  
   **Evidence:** [launcher.rs](core/src/launcher.rs:729) unconditionally says an app’s window opens “on this computer,” although `Bot` supports remote specs and [bot.rs](core/src/bot.rs:317) deliberately treats remote apps differently. Match remote apps before the generic `Kind::App` arm.

No clear credential exposure, unsafe exported Android component, or cleartext default was found. The launcher activity’s `exported=true` is required for its launcher intent.