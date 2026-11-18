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

- **Pick a platform** at the top or on the side. Builder starts that
  platform's dev loop and shows the app running.
- **Ask in the chat.** The agent edits the code. Builder rebuilds and the
  app reloads in the pane, with the tests run on each change.
- **Real data, real calls.** The app signs in to the real switch with a
  test extension that Builder keeps in secure storage (never in code or
  chat). A call can be placed from the pane.
- **One click to ship a build:** an APK for the phone, an iOS build for
  TestFlight later, a deploy of the web app (with confirmation).

## Platforms

| Platform | Code | How you see it | Needs |
|---|---|---|---|
| **Web** | connectix-io/app | The page in the pane (Builder does this today) | Nothing new |
| **Android** | `phone/` | The emulator or a USB phone, mirrored into the pane (scrcpy, Apache-2.0), plus the log | KVM in WSL, or a phone over USB |
| **iOS** | `phone/` | The simulator on a Mac, streamed or as screenshots, plus the log | A Mac (own or rented) |
| **Desktop** (later) | Rust phone or Compose desktop | Its own window, plus the log | Decided after mobile |

## Releases (each one usable on its own)

| Release | You can | Done when |
|---|---|---|
| **1. Platform picker** | Switch between Web and Android in one project; the agent works in that platform's folder; the log and actions follow | The owner switches platforms without a terminal |
| **2. Android live** | See the phone app in the pane and use it; edit by chat, the app reinstalls | A dial pad change shows on the mirrored phone within a minute |
| **3. Real calls** | Sign the phone app in with a stored test extension; place a test call from the pane | Demo 2 (an echo call with audio) runs from Builder |
| **4. iOS** | The same loop on the iPhone simulator, through a Mac | One call from the iOS simulator |
| **5. Desktop** | The desktop phone as a fourth platform | After the mobile app is in daily use |

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

1. The platform picker **at the top or on the side**.
2. **One platform running at a time** (simpler), or Web and Android side by side.
3. **One Builder project spanning both repos** (connectix-io/app and
   builder/phone), or one project per repo.
4. **Android emulator or a USB phone first.**
5. **A Mac for iOS:** own, borrowed, or rented (for example a cloud Mac).
