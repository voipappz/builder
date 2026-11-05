# AGENTS.md — VoipAppz Builder

Read this first. It is the working brief for any coding agent (and person) in
this repo. The longer story is in `docs/appz-process.md` (decisions, status,
session log) and `docs/how-it-works.md`.

## What Builder is (2026-11-05)

The desktop IDE where a customer's **appz** is built by chat. An appz is the
customer's own Elixir app (Connectix, connectix-io/connectix): custom logic,
agents, tools. Builder:

1. signs the user in with their **VoipAppz account** (switch.voipappz.io,
   checked against `/api/mcp`; required; kept in memory only, so the user
   signs in again after a restart),
2. runs the **appz folder** (`builder.toml`: `make setup` once, `make dev
   PORT=…`, health, `stop`, `[[actions]]` such as tests and deploy),
3. gives a coding agent (Claude Code over ACP, `npx
   @agentclientprotocol/claude-agent-acp@0.94.0`) that folder to edit, with
   the appz's dev MCP and read-only VoipAppz API tools,
4. shows the **built-in chat** (left) and the appz's own page (right, e.g.
   `/admin/conversations`), plus a Status tab (log, restart, actions).

The bigger picture, in three facts: each customer gets its own app (PWA);
its screens are our existing React (voipappz/app packages such as
`@voipappz/conversations-ui`) rendered by the customer's Elixir through
live_react; the mothership (voipappz-api + switch) holds users, ACLs, calls
and data. **Keep it simple**: check every proposal against these facts.

Builder edits **only the Elixir appz**. It does not run or edit the React
app; shared screens are changed in voipappz/app and vendored into the appz.

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

1. Merge connectix #4, re-run builder #3, merge it.
2. Merge builder #4; the next tag (v0.1.21) publishes .deb/AppImage.
3. Old `e2e` job: the API in secret `VITE_API_BASE_URL` refuses connections
   (ECONNREFUSED); bring it up or retire the job with the app path.
4. voipappz-api #209 (NATS intake) was closed unmerged; Connectix `main`
   already sends to it.
5. Delete Builder's old app-centric path; Connectix v0.1.25 tag; first run of
   v0.1.20 on Windows.
6. M1b: package the React sip.js phone (`@voipappz/phone-ui`), then the demo
   customer PWA (phone + conversations via live_react, login mocked).
