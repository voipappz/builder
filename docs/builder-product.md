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
| **Android** | `phone/` | The emulator or a USB phone, mirrored with scrcpy (Apache-2.0): its own window first, in the pane later; plus the log | On Windows+WSL, the emulator, adb and scrcpy may run more simply on Windows than inside WSL (KVM, USB passthrough) |
| **iOS** | `phone/` | Build, tests and logs from a named Mac first; the simulator viewed on the Mac itself | A Mac (own or rented); signing and Xcode on it |
| **Desktop** (later) | Rust phone or Compose desktop | Its own window, plus the log | Decided after mobile |

## Releases (each one usable on its own, ordered by risk)

Reviewed by Codex on 2026-11-18 ([codex-builder-product-review.md](codex-builder-product-review.md)):
start with one platform on one device, and add the picker only once two
platforms work.

| Release | You can | Done when |
|---|---|---|
| **1. Android loop** | Open `phone/` (one repo, today's single project); Builder runs the tests, builds, installs on one device (emulator or USB phone), launches the app and opens scrcpy **in its own window** | A chat edit is tested, installed and seen on the phone without a terminal |
| **2. Reliable reload** | Rebuild, reinstall and relaunch on every change, with the phone's log (logcat) in Builder's log | A change shows on the phone within a minute, every time |
| **3. Phone in the pane** | The mirrored phone inside Builder, after a spike proves embedding works (WebView/Tauri cannot simply frame a native window) | The spike passes on Windows/WSL |
| **4. Platform picker** | Switch between projects (Web = connectix-io/app, Android = `phone/`), one project per repo, one agent per project | Two working platforms, switched in one click |
| **5. Call shortcut** | A button to place a test call, once the same APK already calls on its own (phone gate 2) | Builder only installs, launches and shows logs; the phone keeps its own login |
| **6. iOS build and test** | Build and run tests on a named Mac, logs back in Builder; the simulator stays on the Mac's screen | One iOS test run from Builder |
| **7. iOS viewing** | The simulator shown in Builder (an experiment) | Usable latency |
| **8. Desktop** | The desktop phone as another platform | After the mobile app is in daily use |

## Not in scope now

Customers building their own apps; a marketplace; hosting; Builder on
mobile; arbitrary projects that are not VoipAppz apps.

## How we know it works

- The owner uses Builder, not a terminal, for a full week of phone work.
- A change is visible on the chosen platform within a minute.
- Switching platforms takes one click and under a minute.

## Risks

- **iOS needs a Mac.** Without one, release 4 waits; the rest does not.
- **Emulators in WSL** need KVM, and they are slow; a USB phone is the fallback.
- **Scope creep** back into the "customer appz" product. This plan is for
  personal use first.

## Open decisions for the owner

1. **Android emulator or a USB phone first,** and whether adb, the emulator
   and scrcpy run on Windows or inside WSL.
2. **A Mac for iOS:** own, borrowed, or rented (for example a cloud Mac).
3. Later, after two platforms work: the picker **at the top or on the side**,
   and one platform at a time or side by side.

Recommended (Codex): **one Builder project per repo**; the picker switches
projects, and no agent gets both repos at once.
