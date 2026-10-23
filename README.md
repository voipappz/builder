# VoipAppz Builder

**Build your custom PBX through chat, see it running, and refine it through chat.**

VoipAppz Builder is a planned desktop coding environment for custom PBX
development on the VoipAppz platform. A customer describes a change in chat.
The Builder agent in the VoipAppz API reads and edits their React project,
validates it, and shows the result in separate Admin and User previews. The
customer reviews the diff and saves a revision.

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
# 1. A worktree of the app for the agent to work in, with the chat client
git -C ../app worktree add ../app-builder feat/builder-client
# 2. The app's dev server in that worktree (serves the chat and the previews)
(cd ../app-builder && npm run dev)
# 3. Optional: the local API's MCP tools, as `make claude` does
#    export VA_MCP_BASIC=$(cd ../voipappz-api && MCP_AUTH=1 bin/dev-status.sh)
# 4. Builder
cargo run -p builder-cli -- --workspace ../app-builder \
  --client-url http://localhost:3000 [--api-url http://127.0.0.1:5055]
```

Open the printed `http://127.0.0.1:9832/?secret=…` address. Claude Code
must be signed in (`claude`) on this machine.

## Start here

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
- The Ruby API holds the MCP client. The React project hosts a dev-only MCP
  server. The Builder desktop app relays between them.
- The first API target is `https://switch.voipappz.io`.

## Related repositories

| Repository | Role |
|---|---|
| `voipappz/app` | React/Vite admin app, the customer project template, the chat UI package and the React MCP server |
| `voipappz-api` | Builder agent, project/session/run API, MCP client, platform tools |
