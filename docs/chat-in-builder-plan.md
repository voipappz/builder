# Plan: Builder's chat lives in the Builder repo

Written 2026-11-19. The owner decided: Builder's chat moves into this repo,
following Tauri's usual setup (the frontend in the app's own repo, built with
Vite; Tauri's templates offer plain HTML or React).

## Why

- Today the chat is built from another repository (voipappz/app,
  `src/builder-client` and `packages/chat-ui`). Every installer build clones
  it with a deploy key (`scripts/client.sh`, `client.ref`), and a chat change
  needs a PR in that repo first.
- The app itself never uses `src/builder-client`. It does use `chat-ui` for
  its own AI chat, so the move takes a copy of `chat-ui`, and the app keeps its
  own.
- The screen has problems for a native app project (the phone): a voice-bot
  "Bot" tab with "Hold to talk", a long error line that overflows, bot wording
  ("Explain this bot"), and "Disconnected from Builder. Reload to reconnect"
  with no automatic reconnect.

## Decision: React + Vite, moved as it is

React and plain HTML are both standard for Tauri. The chat already works in
React (about 1,100 lines plus `chat-ui`), so moving it costs far less than a
rewrite and keeps its tests. Plain HTML stays as it is for the small setup
screen (`desktop/ui/index.html`).

## Layout

```
chat/                      the chat frontend (React 19, MUI, Vite)
  package.json, package-lock.json, vite.config.js, vitest.config.js
  src/main.jsx             entry: reads the page's voipappz-builder:info meta tag
  src/BuilderApp.jsx, BotPanel.jsx, useBuilderChat.js, acpConnection.js,
      phoenixSocket.js, chatState.js, botCall.js, captureWorklet.js
  src/chat-ui/             copy of voipappz/app packages/chat-ui (MIT, VoIPappz)
  src/theme/               the theme, with RustSIPPhone's colours (below)
client/                    build output (gitignored), embedded by core/build.rs as today
```

- `npm run build` in `chat/` writes `client/` (client.js, client.css,
  assets). `core/build.rs` keeps embedding it, so the server code
  (`/builder/*`, the page's CSP and nonce) does not change.
- Dev: `make chat-dev` runs Vite with hot reload; Builder's `--client-url`
  points at it, as it does today with the app's dev server. The module path in
  `builder.rs` changes from `src/builder-client/main.jsx` to `src/main.jsx`.
- CI and installers build `chat/` directly: no clone and no deploy key.
  `scripts/client.sh` and `client.ref` are removed.

## Look: RustSIPPhone's style

The desktop phone (RustSIPPhone, egui, MIT) is the reference for the look.
Its tokens become the theme's dark palette:

| Token | Colour | Use |
|---|---|---|
| BG | `#12151A` | page background |
| SURFACE | `#1B2027` | panels, the chat input |
| SURFACE_HI | `#262D37` | hover, selected tab, chips |
| TEXT | `#ECEFF4` | text |
| MUTED | `#8C96A5` | secondary text |
| ACCENT | `#4C8DFF` | buttons, links, focus |
| GREEN | `#2FB35A` | ready, success |
| RED | `#E5484D` | errors, stopped |
| AMBER | `#E0A030` | warnings, starting |

Corners 10 to 14 px, text 13 to 16 px, spacing 8 by 6 px. Dark only at first.

## Fixes made during the move

1. **The right pane follows the project kind.** A project with a `page` (web
   app or phone) shows that page first, plus a Status tab (status, restart,
   actions, log). "Hold to talk" appears only for `kind = "bot"`.
2. **Wording per kind:** "app" or "product" instead of "bot"; suggestions per
   kind (phone: explain the app, run the tests, change the sign-in screen).
3. **Long lines wrap** in errors and action results (`overflow-wrap: anywhere`).
4. **The chat reconnects by itself** after Builder restarts the agent or the
   socket drops, with a short backoff, and says "Reconnecting…" meanwhile.

## Steps (each is its own PR)

1. **Move:** `chat/` with the code as it is, its tests, `npm run build` into
   `client/`; CI and Installers build it and drop the clone. Done when the
   installer's chat looks and works as in v0.1.29.
2. **Fixes 1 to 4** above, with unit tests (vitest) for the kind logic and the
   reconnect.
3. **The look:** RustSIPPhone's palette and spacing in the theme.
4. **Remove the old copy:** a PR in voipappz/app deleting `src/builder-client`
   and `vite.builder.config.js` (the app keeps `chat-ui`).
5. **Release** (v0.1.30 or later).

## Checks

- `chat/`: vitest (the existing chatState and botCall tests, plus new ones),
  run by CI's rust job.
- Rust: the existing tests that `/builder/client.js` is served with the CSP.
- e2e: the page check (`scripts/e2e-appz-page.cjs`) still loads the chat.

## Open

- A light theme, and Hebrew, later if wanted.
- Who keeps `chat-ui` in step: the Builder copy and the app's copy can drift.
