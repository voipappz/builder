# VoipAppz Builder

**Build your custom PBX through chat, see it running, and refine it through chat.**

VoipAppz Builder is a desktop coding environment for the VoipAppz React app.
You describe a change in chat; a coding agent (Claude Code, on your own
account) edits the app in Builder's own worktree, and the result shows in
separate Admin and User previews next to the chat.

**Status: first version.** A local server (`core/`, `cli/`; see
[NOTICE](NOTICE) for its origin) serves a page that loads the chat from the
app and runs Claude Code on an app checkout. The desktop app (`desktop/`)
runs it from the tray.

## Install

- **Desktop app:** download the installer for your system from the latest
  [Installers](../../actions/workflows/installers.yml) run (artifacts) or a
  release: `.deb`/`.AppImage` (Linux), `.exe` (Windows), `.dmg` (macOS).
  On first start it asks for the app checkout to work in; settings live in
  `~/.config/voipappz-builder/builder.toml` (Linux). Not code-signed yet.
- **From this checkout, Linux, no sudo:** `install/install.sh` adds
  VoipAppz Builder to the desktop menu; `install/install.sh --uninstall`
  removes it.

## Run it (development)

```text
cargo run -p builder-cli -- --app-repo ../app --start-app --api-dir ../voipappz-api --open
```

Builder works on the app repository (`../app`) in its own git worktree,
`../app-builder` on branch `builder/work`, created from `origin/main` on first
start, so your own checkout is never touched. It links `node_modules` from
`../app` (or runs `npm ci`), starts the app's dev server there, which serves
the chat and the previews, and, when the local API answers, gives the agent
its `/api/mcp` tools with the login `make claude` uses. The agent edits files
without asking (its "Accept edits" mode) and asks before running commands.

Claude Code must be signed in (`claude`) on this machine.

## Start here

- [How it works](docs/how-it-works.md): the parts, start-up, the rules the
  server enforces, settings, running and releasing.
- [PLAN.md](PLAN.md): product boundary, delivery steps, MCP topology, installer
  flow, branding and acceptance checks.
- [Assistant dependency audit](docs/assistant-dependency-audit.md): the current
  Assistant, VML and Bot coupling in the app and API, which Builder must not use.
- [Upstream reference](docs/upstream-reference.md): what was learned from the
  desktop app the local server started from.
- [Example project descriptor](examples/builder-project.example.json): a design
  fixture for the planned parser, not a runnable configuration.

## Boundaries

- The chat UI comes from the app's shared package `@voipappz/chat-ui`, which
  draws the chat and holds no data logic.
- Builder has no connection to the app's Assistant, VML or Bot code, endpoints,
  storage or libraries.
- The agent writes code only in the app worktree; the server refuses writes
  anywhere else.
- The VoipAppz API is read-only for the agent: Builder proxies its MCP tools
  and refuses the ones that change the platform.
- The first API target is the local API (`voipappz-api`, `make dev`).

## Related repositories

| Repository | Role |
|---|---|
| `voipappz/app` | The React app Builder works on; the chat client (`src/builder-client/`) and the chat UI package (`packages/chat-ui`) |
| `voipappz/api` | The VoipAppz API; its `/api/mcp` tools reach the agent read-only; the vendored contracts |
