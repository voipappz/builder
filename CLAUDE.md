# CLAUDE.md — VoipAppz Builder

Read this first. It is the current brief for any coding agent (and person) in
this repo. Details: `AGENTS.md` (Builder's layout, rules and history; its
"What Builder is", "Customer apps" and "Open" sections describe the earlier
appz concept, superseded on 2026-11-17) and `phone/docs/state.md` (where the
phone work stands and what is next).

## The project now (2026-11-18)

| Part | What | State |
|---|---|---|
| **Builder** (`core/`, `cli/`, `desktop/`) | Rust + Tauri desktop tool: a local server whose chat page drives a coding agent (Claude Code over ACP) with read-only VoipAppz API tools, and runs a project folder described by `builder.toml` | Released up to v0.1.32. Builds the VoipAppz apps per platform (Web, Mobile, later Desktop): setup asks which; the phone project runs the Android emulator beside the chat. Its chat is in `chat/` (React + Vite). Plans: `docs/builder-product.md`, `docs/chat-in-builder-plan.md` |
| **VoipAppz Phone** (`phone/`) | Mobile softphone in Kotlin Multiplatform (Android first, iPhone later): our own SIP-over-WebSocket code, webrtc-kmp audio, Compose Multiplatform | **Current focus.** Registration code done; the switch's 401 challenge reached; calls next. Plan: `phone/docs/plan.md` |
| **Desktop phone** | Later, from the Rust fork `voipappz/RustSIPPhone` (outside this repo) | After the mobile app; compare a Compose desktop app first (the fork has no WebRTC audio) |

The order: **mobile first with Kotlin Multiplatform, then desktop.**

## Decided (do not re-open without the owner)

- The phone uses **SIP over WebSocket** to `wss://switch.voipappz.io:8443`
  (FreeSWITCH), like the browser phone (sip.js). Not Verto.
- **Kotlin Multiplatform**, shared code for Android and iPhone.
- **Permissive licences only** (MIT, Apache, BSD); no GPL/AGPL/paid.
- **Native**, not a web page in a shell; **no native SIP libraries** (no
  baresip, PJSIP, Linphone). The SIP code is ours, modelled on sip.js.
- **Tauri is only for Builder**, not for the phone app.
- The phone app's **design is open for discussion**: ask, do not invent.
- **No Elixir.** The Elixir appz (Connectix, connectix-io/connectix) is not
  used: do not clone it, test against it or plan on it. Docs that describe it
  (`docs/appz-plan.md`, `docs/appz-process.md`, `docs/demo.md`, `docs/vision.md`,
  `docs/builder-plan.md`, parts of `AGENTS.md`) are history.
- **Keep it simple**: the smallest change that does the job; short answers.

## Commands

```sh
# Builder (Rust)
export PATH=$HOME/.cargo/bin:$PATH
cargo test                        # core + cli
cargo run -p builder-cli -- --bot-dir phone --api-url https://switch.voipappz.io --open

# Phone (Kotlin; tools from phone/mise.toml, every target runs through mise exec)
cd phone
make setup                        # once
make test | make apk | make install | make probe | make registercheck
```

CI: `.github/workflows/test.yml` (Builder: rust with the chat's tests and
build, contracts, e2e), `installers.yml` (Builder installers; release on `v*`
tags), `phone.yml` (phone: Android on Linux, the mobile dev environment with
the emulator, Builder with the phone project end to end in a browser
(`scripts/e2e-phone.sh`), iOS simulator on macOS; runs on changes to `phone/`,
`core/`, `cli/` or `chat/`).

## Rules

- **Secrets:** never read or print `.env` files or credential files; never
  put an extension, password, token or real phone number in code, tests, docs,
  memory or chat. Tests use fake values (`1001`, `secret`, `pbx.example.com`).
  SIP test credentials come from the environment or a file the owner fills
  (`~/.config/voipappz-phone/test.env`); never open it.
- **This repo is public:** no personal paths, internal hostnames, switch
  configuration or customer data in code or docs.
- **Prove before building:** one gate at a time (`phone/docs/plan.md`); a gate
  is done when proven against the real switch, not just in tests.
- **Branches and PRs; the owner merges.** Do not touch the user's main
  checkouts; confirm tags, releases, deploys and anything outward first.
- **Reviews:** get a Codex review of substantial changes, save it under
  `phone/docs/research/notes/` (phone) and fix what it finds.
- Kotlin: no Java/Android APIs in `commonMain`/`commonTest`; versions in
  `phone/gradle/libs.versions.toml`; tools pinned in `phone/mise.toml`.
- Rust/Builder: Windows code is compiled only by Installers CI; no "tidewave"
  naming in our code (dev MCPs are `voipappz-*` at `/voipappz/mcp`).
