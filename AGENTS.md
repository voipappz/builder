# AGENTS.md — VoipAppz Builder

Read this first. It is the working brief for any coding agent (and person) in
this repo. The longer story is in `docs/appz-process.md` (decisions, status,
session log) and `docs/how-it-works.md`.

## What Builder is (2026-11-10)

**The target: a customer builds their own application using Builder.** The
plan is `docs/appz-plan.md`; read it before proposing anything. In short:

- An **appz** is one container image per customer in VoipAppz's cloud:
  Connectix (the Elixir engine: voice bot, phone calls over its SIP user
  agent and mod_earshot, flows, events), the React widget catalog from
  voipappz/app rendered through live_react, and the **customer folder**
  (Elixir only: LiveViews, handlers, tools, flows, bot, theme), pulled at
  boot and on every push.
- Builder signs the user in with their **VoipAppz account**, starts an appz
  from the template, gives the coding agent (Claude Code over ACP) the
  customer folder with read-only VoipAppz API tools, pushes the folder to
  the customer's cloud dev instance, shows the built-in chat (left) and the
  appz page (right), lets the user talk to the bot (the Bot tab plays the
  switch against `/ws/earshot`), and deploys through the mothership.
- The mothership (voipappz-api + switch) holds users, ACLs, calls, numbers
  and data. No database in an appz. Customers never write React, never
  install a toolchain, never see engine code.

Today Builder still runs a local folder (`builder.toml`: `make setup`,
`make dev PORT=…`, health, `stop`, `[[actions]]`); the cloud dev instance
and "New appz" are steps 8 and 9 of the plan. **Keep it simple**: check
every proposal against the plan's three layers.

## Layout

| Path | What |
|---|---|
| `core/src/builder.rs` | Local server on 127.0.0.1: the page, `/builder/*` (built-in chat files), the Phoenix socket for ACP/MCP, the read-only API MCP proxy |
| `core/src/launcher.rs` | Settings, pre-flight checks, the start flow, `appz_only` |
| `core/src/bot.rs` | Runs the appz folder: setup, dev, health, actions, stop |
| `core/src/ws/` | ACP (the agent) and MCP channels |
| `core/src/wsl.rs` | Windows → WSL paths and commands |
| `core/build.rs` | Embeds `client/` (the chat bundle) into the binary |
| `cli/` | `voipappz-builder` CLI (same start flow; used by the e2e scripts) |
| `desktop/` | Tauri tray app; onboarding page `desktop/ui/index.html` |
| `client.ref` | voipappz/app ref the chat is built from (`main`) |
| `client/` | Built chat (gitignored), from `scripts/client.sh` |
| `scripts/client.sh` | Clones voipappz/app at `client.ref` into `.client-app/`, runs `npm run build:builder`, copies `dist-builder` to `client/` |
| `scripts/e2e.sh` | Old app-path end to end (React app + `make dev`) |
| `scripts/e2e-appz.sh`, `scripts/e2e-appz-page.cjs` | Appz end to end: Connectix only, the page checked in Chromium |
| `contracts/` | ACP/MCP contract tests (Node) |
| `.github/workflows/test.yml` | rust, contracts, e2e, e2e-appz |
| `.github/workflows/installers.yml` | Windows .exe (+ Linux .deb/AppImage after #4); release on `v*` tags |

## Commands

```sh
export PATH=$HOME/.cargo/bin:$PATH        # cargo is not on PATH by default
scripts/client.sh                          # build the chat into client/ (before cargo build, for a real page)
cargo test                                 # core + cli; the desktop crate needs GTK/WebKit (CI builds it)
cargo run -p builder-cli -- --bot-dir <appz folder> --api-url https://switch.voipappz.io --url-file <file>
CONNECTIX_DIR=<appz folder> NODE_PATH=$PWD/.client-app/node_modules scripts/e2e-appz.sh
```

**Release:** bump `version` in `Cargo.toml` and `desktop/tauri.conf.json`,
run `cargo update --workspace` (Cargo.lock), merge, tag `vX.Y.Z`, push the
tag; Installers publishes the release. Check that the tag's run started.

## Rules

- Never read or print `.env` / `.env.token` contents; never write
  credentials to the repo, docs or memory; never paste keys in chat.
- The agent writes only inside its folder; API tools are read-only; logins
  are filtered from the agent's (and WSL's) environment.
- Every request needs the launch secret, a Host naming this server and, when
  present, a matching Origin. Only `GET /builder/*` (public chat files)
  skips the secret.
- Work in worktrees and PRs; do not touch the user's main checkouts
  (`~/voipappz/app`, `~/connectix/app`, voipappz-api); leave `../mobile`
  alone. Confirm merges, tags, deploys and anything outward first.
- No "tidewave" naming in our code; dev MCPs are `voipappz-*` at
  `/voipappz/mcp`.
- Windows code is compiled only by Installers CI: run it (`gh workflow run
  installers.yml --ref <branch>`) before merging desktop changes.

## Work done 2026-11-04/05

| What | Where | State |
|---|---|---|
| Chat built from voipappz/app (`build:builder`), embedded, served at `/builder/*` | app #301, builder #1 | merged, **v0.1.19** |
| Appz-only mode (no React app, no previews, agent via npx in the appz) | builder #1 | **v0.1.19** |
| Onboarding: sign in with your VoipAppz account first (required), then the appz folder | builder #1 | **v0.1.19** |
| Tray and setup-window wording for the appz | builder #2 | **v0.1.20** (latest) |
| CI `e2e-appz` (deploy key `voipappz/builder CI`, secret `CONNECTIX_DEPLOY_KEY`) | builder #3 | open; waits for connectix #4 |
| Connectix `make setup` builds the admin assets (`make admin-ui`, Docker only), found by e2e-appz | connectix #4 | open |
| Linux packages (.deb for Ubuntu 22.04+, AppImage) in Installers | builder #4 | open; CI run passed (Linux and Windows) |
| Decisions: sip.js phone; conversations over NATS to the mothership (Mnesia = live copy); end-user login mocked for M1b; demo on a real VoipAppz account | `docs/appz-process.md` | session log on builder #3's branch |

## Open

1. Merge builder #7 (voipappz/app is private: clone it with the deploy key
   `APP_DEPLOY_KEY`); until then do not tag a release.
2. Old `e2e` job: the API in secret `VITE_API_BASE_URL` refuses connections
   (ECONNREFUSED); retire it with the app path or point it at a live API.
3. Plan steps 1–4 (`docs/appz-plan.md`): the live call over Parrot from the
   cloud Connectix; needs SIP credentials on the instance (owner).
4. Connectix branch `dev/native-loop` (worktree `~/connectix/app-local`,
   commit ea24b76, not pushed): `MODE=native` via mise for setup/dev/stop/
   test; verified. Push and PR when the owner says.
5. Builder for the cloud model: remote-product mode (page + socket URL +
   token from the mothership), "New appz" from the template, push to the
   dev instance, setup progress in the main window, appz wording in the
   chat's suggestion chips.
6. M1b: package `@voipappz/phone-ui` (sip.js); `dashboard-ui` in shadcn with
   the port of `chat-ui`/`conversations-ui` (Codex: at dashboard-ui time).
