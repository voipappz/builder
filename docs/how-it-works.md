# How VoipAppz Builder works

VoipAppz Builder lets you change the VoipAppz React app by chatting with a
coding agent, and see the result running next to the chat.

## The parts

```text
Browser tab  http://127.0.0.1:9832/?secret=…
│
├─ page shell ............ Builder's local server
├─ chat screen ........... ../app code (src/builder-client, packages/chat-ui),
│                          served by the app's Vite dev server (localhost:3000)
├─ Admin / User previews . the same dev server on admin.localhost / user.localhost
│
└─ websocket (ACP) ─────▶ Builder's local server
                              ├─▶ Claude Code (started by Builder) in ../app-builder
                              └─▶ /mcp/api ─▶ VoipAppz API /api/mcp (read-only)
```

| Part | Code | Job |
|---|---|---|
| Local server | `core/src/builder.rs` | Serves the page, carries the chat to the agent, enforces the rules below |
| Start-up | `core/src/launcher.rs` | Settings, worktree, dev server, API login, starts the server |
| CLI | `cli/` (`voipappz-builder`) | Starts Builder from a terminal |
| Desktop app | `desktop/` | Tray icon that starts Builder and opens the browser |
| Chat screen | `../app` `src/builder-client/` | `ChatView` over ACP, permission bar, agent modes, previews |
| Agent | Claude Code via `@agentclientprotocol/claude-agent-acp`, from the app's own packages (`bin/npm.sh exec -- claude-agent-acp`; `npx` until the app has it) | Reads and edits the app, on the user's own Claude account |

The chat screen is app code, so a chat change ships with the app. The local
server does what a web page cannot: start programs and read files. It runs
no Node itself; everything Node is the app's Makefile.

## What happens on start

1. Read settings and the user's login-shell environment (so `make` and the
   app's tools are found when started from a menu), then run the pre-flight
   checks: app folder, `git`/`make`/`bash`/`curl`, disk space, ports, API.
2. Use Builder's own worktree of the app repository: `../app-builder` on
   branch `builder/work`, created from `origin/main` the first time. Your own
   `../app` checkout is never touched. If its packages are not installed,
   run `make setup` there.
3. Write the worktree's `.env` once so the app's API proxy uses the same API
   as the agent (an existing `.env` is never changed).
4. Run `make dev PORT=<free port>` in the worktree if the chat address does
   not answer.
5. If the API answers and the login works, offer its MCP tools read-only.
6. Start the server on 127.0.0.1 with a new launch secret and open the page.

## Rules the server enforces

- **Launch secret.** Every request needs the per-launch secret (`?secret=` or
  `x-builder-secret`), a Host naming the server and, when sent, an Origin
  naming it. The secret changes on each start; do not share the address.
- **Only three routes.** `/`, `/socket/websocket` and `/mcp/api`. The
  upstream file, shell, proxy, download and terminal features are not
  reachable.
- **The server chooses what runs.** The agent command, its folder and its MCP
  servers come from Builder's settings; whatever the page sends is ignored.
- **Code is written only in the app.** The agent's "Accept edits" mode lets
  it edit inside the worktree without asking. A request to edit, delete or
  move a file outside the worktree, inside its `.git`, or through a symlink
  that leads out is refused by the server and never reaches you. Commands
  still ask you each time.
- **The API is read-only.** The agent reaches `/api/mcp` only through
  Builder's `/mcp/api`. Builder adds the login (which the agent and browser
  never see), hides tools that change the platform (marked destructive, or
  named `create`, `update`, `delete`, `buy`, …) and refuses calls to them.

## Settings

| Where | Used by |
|---|---|
| `~/.config/voipappz-builder/builder.toml` | Desktop app (Linux; the OS config folder elsewhere) |
| `~/.config/voipappz-builder/builder.env` | `install/install.sh` and `voipappz-builder-launch` |
| `~/.local/state/voipappz-builder/` | Logs: `builder.log`, `app.log` (dev server) |

`builder.toml` keys: `app_repo`, `workspace` (default `<app_repo>-builder`),
`client_url` (default `http://localhost:3000`), `preview_url`, `api_url`
(default from `api_dir`'s `.env` `VA_WEB_PORT`), `api_dir`, `start_app`,
`port` (9832), `agent_command`.

The API login is `VA_MCP_BASIC` if set, else what `MCP_AUTH=1
bin/dev-status.sh` prints in `api_dir`, as `make claude` does.

## Running it

| Way | How |
|---|---|
| Installer | Download from the [releases](https://github.com/voipappz/builder/releases); first start asks for the app repository |
| Linux, no sudo | `install/install.sh`, then the "VoipAppz Builder" menu entry or `voipappz-builder-launch [start\|stop\|status\|logs]` |
| Terminal | `cargo run -p builder-cli -- --app-repo ../app --start-app --api-dir ../voipappz-api --open` |

Claude Code must be installed and signed in (`claude`). For the API tools,
run `make dev` and `make tenant` in `voipappz-api`.

## Building and releasing

- `cargo test` runs the core and CLI tests; `npm test` in `contracts/` runs
  the contract fixtures. CI: `.github/workflows/test.yml`.
- `scripts/e2e.sh` (CI job `e2e`) is the user's path end to end: it clones
  voipappz/app, starts the CLI with `--start-app` (worktree, `make setup`,
  `make dev`), then checks the Builder page, the app and the agent's ACP
  `initialize`. It needs `TEST_EMAIL` and `TEST_PASSWORD` for the app's login.
- `.github/workflows/installers.yml` builds the desktop app for Linux
  (`.deb`, `.AppImage`), Windows (`.exe`) and macOS (`.dmg`) on pushes to
  `main` that touch the app, or on demand. A `v*` tag also creates a draft
  release with the installers; publish it with `gh release edit <tag>
  --draft=false`.
- Before tagging, set the same version in `Cargo.toml` and
  `desktop/tauri.conf.json`.
- Installers are not code-signed yet, and the desktop app has no updater.

## Changing the chat

Edit `src/builder-client/` in the app (or in `../app-builder` to see it live:
the dev server reloads it). It needs only lint locally; its unit tests run in
the app's CI. The ACP updates it understands are listed in `chatState.js`.
