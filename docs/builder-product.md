# Builder as a product: one place to build VoipAppz apps on every platform

Written 2026-11-18. Replaces the "appz for customers" concept in
`vision.md`, `builder-plan.md` and `appz-plan.md` (superseded 2026-11-17).
A draft for the owner: the decisions at the end are open.

## The product in one sentence

**Builder is a desktop app where you chat with a coding agent and watch
your VoipAppz app change live, on the web, on Android and on iPhone (later
desktop), against the real switch.**

## Who it is for

1. **First: the owner, for personal use.** Building VoipAppz's own apps:
   the web app (connectix-io/app) and the phone app (`phone/`, Kotlin
   Multiplatform).
2. **Later, maybe:** the VoipAppz team; then partners who build on
   VoipAppz. Not planned now.

## The problem

Building one product for several platforms means juggling a terminal per
platform, an emulator, Xcode on a Mac, a browser, a SIP account and an
agent in yet another window. Each one is set up by hand, and checking that
a change works everywhere takes longer than making it.

## The experience

```
┌────────────────────────────────────────────────────────────────┐
│ VoipAppz Builder   [ Web | Android | iOS | Desktop ]  ● running │  ← platform picker (top or side)
├──────────────────────┬─────────────────────────────────────────┤
│ Chat with the agent  │  The app, live:                          │
│                      │   Web: the page                          │
│ "make the dial pad   │   Android: the phone screen (emulator    │
│  bigger"             │            or USB phone), mirrored       │
│                      │   iOS: the simulator, from a Mac         │
│                      │                                          │
│                      ├─────────────────────────────────────────┤
│                      │  Log · Tests · Actions (Build, Install)  │
└──────────────────────┴─────────────────────────────────────────┘
```

- **Pick a platform** at the top or on the side (from release 4). Builder
  starts that platform's dev loop and shows the app running.
- **Ask in the chat.** The agent edits the code. Builder rebuilds and the
  app reloads in the pane, with the tests run on each change.
- **Real data, real calls.** The phone app signs in to the real switch with
  its own login (its secure storage, not Builder's). Once the phone calls on
  its own, Builder adds a test-call shortcut.
- **One click to ship a build:** an APK for the phone, an iOS build for
  TestFlight later, a deploy of the web app (with confirmation).

## Platforms

| Platform | Code | How you see it | Needs |
|---|---|---|---|
| **Web** | connectix-io/app | The page in the pane (Builder does this today) | Nothing new |
| **Android** | `phone/` | The Android emulator on the PC (in WSL, with KVM), shown **inside Builder's pane**: the emulator streams its screen to a web page (Google's emulator WebRTC stream, or ws-scrcpy), which the pane shows like any page; touch and keys go back. Plus the log | The PC only (no phone, no cable); the user in the `kvm` group |
| **iOS** | `phone/` | **Cannot run on a PC:** Apple's simulator runs only on macOS. On the PC, Builder shows the iOS build and test results from CI (GitHub's macOS runner already builds and tests it); viewing the simulator inside Builder needs a remote Mac streamed into the pane, later | A cloud Mac, only for viewing |
| **Chrome extension** | A browser extension project (to be created) | The extension's page or popup in the pane; loading it into Chrome itself comes later | A project folder with a `builder.toml` |
| **Desktop** (later) | Rust phone or Compose desktop | Its own window, plus the log | Decided after mobile |

## Releases (each one usable on its own, ordered by risk)

Reviewed by Codex on 2026-11-18 ([codex-builder-product-review.md](codex-builder-product-review.md)):
start with one platform on one device, and add the picker only once two
platforms work.

| Release | You can | Done when |
|---|---|---|
| **1. Spike: the emulator inside Builder** | Start the Android emulator on the PC and show its screen in Builder's pane through a web stream; check licences (permissive only), latency and touch | The emulator is usable inside Builder |
| **2. Android loop** | Open `phone/` (today's single project); Builder runs the tests, builds, installs on the emulator, launches the app and shows it in the pane | A chat edit is tested, installed and seen inside Builder without a terminal |
| **3. Reliable reload** | Rebuild, reinstall and relaunch on every change, with the phone's log (logcat) in Builder's log | A change shows within a minute, every time |
| **4. Platform picker** | Switch between projects (Web = connectix-io/app, Android = `phone/`), one project per repo, one agent per project | Two working platforms, switched in one click |
| **5. Call shortcut** | A button to place a test call, once the same APK already calls on its own (phone gate 2) | Builder only installs, launches and shows logs; the phone keeps its own login |
| **6. iOS build and test** | Builder shows the iOS build and test results from CI's macOS runner (no Mac on the PC) | The iOS result of each change visible in Builder |
| **7. iOS viewing** | The simulator of a cloud Mac streamed into Builder's pane (an experiment, if wanted) | Usable latency |
| **8. Desktop** | The desktop phone as another platform | After the mobile app is in daily use |

## Not in scope now

Customers building their own apps; a marketplace; hosting; Builder on
mobile; arbitrary projects that are not VoipAppz apps.

## How we know it works

- The owner uses Builder, not a terminal, for a full week of phone work.
- A change is visible on the chosen platform within a minute.
- Switching platforms takes one click and under a minute.

## Risks

- **iOS needs a Mac.** On the PC only builds and tests (through CI); viewing waits for a cloud Mac.
- **The emulator in WSL** needs KVM and memory; the stream into the pane is the spike's job to prove.
- **Scope creep** back into the "customer appz" product. This plan is for
  personal use first.

## Open decisions for the owner

Decided 2026-11-18: **everything on the PC, inside Builder** (the Android
emulator, not a phone; iOS through CI until a Mac is wanted).

1. **iOS viewing:** whether to rent a cloud Mac later, or keep iOS to CI
   results.
2. Which emulator stream the spike picks (Google's WebRTC stream or ws-scrcpy).
3. Later, after two platforms work: the picker **at the top or on the side**,
   and one platform at a time or side by side.

Recommended (Codex): **one Builder project per repo**; the picker switches
projects, and no agent gets both repos at once.
