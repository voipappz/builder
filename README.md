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

## Windows: install and make your first change

Builder is a pre-release for developers. The installer is not code-signed yet
and has no automatic updates.

**Before you start**

- Windows 10 or 11, 64-bit, with internet access for the first run.
- A Git checkout of the VoipAppz React app (`voipappz/app`) **on a local
  drive** (for example `C:\Users\you\voipappz-app`). A folder on WSL
  (`\\wsl.localhost\...`) or a network share does not work: `npm` on Windows
  cannot run there. To work on a WSL checkout, run the Linux Builder inside
  WSL instead (see "Run it" below); the Windows browser reaches it on
  `localhost`. Builder makes its own worktree next to the checkout,
  `<app folder>-builder`, on the branch `builder/work`.
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed and
  signed in (run `claude` once in a terminal). The agent runs on your Claude
  account.
- A VoipAppz account on `https://switch.voipappz.io` to sign in to the
  previews. Node.js is not required: Builder installs the version the app needs.

**Install**

1. Download `VoipAppz.Builder_<version>_x64-setup.exe` from the
   [releases](https://github.com/voipappz/builder/releases).
2. Run it. If Windows shows **Windows protected your PC**, select **More
   info**, then **Run anyway**.
3. Start **VoipAppz Builder** from the Start menu. It runs from the system
   tray and opens a setup window.

**First start**

1. Under **React app**, choose the folder of your `voipappz/app` checkout.
2. Leave the **API address** as `https://switch.voipappz.io`, or enter another
   HTTPS address.
3. Optional: enter your API email and password so the agent gets read-only
   platform tools (extensions, numbers, call logs). Leave both empty to skip.
4. Select **Check**; you should see **API is reachable**.
5. Select **Start**. The window shows each step: installing Node, installing
   the app's packages, starting the app, starting the agent. The first run can
   take several minutes.
6. When setup finishes, your browser opens the Builder page: chat on one side,
   Admin and User previews on the other. Sign in to each preview with your
   VoipAppz account.

**Make your first change**

1. In the chat, describe the change, for example: *Add a search field above
   the extensions table.*
2. The agent reads the app's source and edits it. Edits inside the worktree
   are applied without asking; a command the agent wants to run asks first.
3. The previews reload on their own. Check the Admin preview, then ask for a
   refinement or say what to undo.
4. Your changes are files in the worktree on `builder/work`. Review them with
   Git, then commit and push or merge as usual.

**Where things are**

- Settings: `%APPDATA%\voipappz-builder\builder.toml`.
- Logs: `%LOCALAPPDATA%\voipappz-builder\builder.log` and `app.log`.
- Tray menu: **Open VoipAppz Builder**, **Connect App and API…**,
  **Settings**, **View Logs**, **Restart**, **Quit VoipAppz Builder**. Closing
  the setup window leaves Builder running in the tray.

**If it doesn't work**

- "Use an HTTPS API address": only localhost may use plain `http://`.
- "Could not reach the API": check the address in a browser and your
  connection.
- Setup stops with an error: the window keeps the details and the log
  location. Use **Copy details** when reporting it.
- The chat does not answer: make sure Claude Code is signed in (`claude` in a
  terminal) and your Claude account has quota left.
- The previews are empty: another program may be using port 3000. Stop it and
  **Restart** Builder from the tray.

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

Builder checks Node and npm at startup. If either is missing, or Node is too
old, it uses mise to install the Node version in the app's
`mise.toml` (22 by default). The resulting PATH is shared by package installation,
the dev server, and the agent. If mise is missing, Builder downloads its
[official installer](https://mise.jdx.dev/installing-mise.html) and installs a
private copy under Builder's state directory. First-run setup requires internet
access; no preinstalled npm or mise, administrator access, or shell activation
is needed. Setup progress and download/install failures appear in the launcher.

The desktop app shows a setup window with the current step, elapsed time and
recent progress. Long installs report every five seconds. If setup fails, the
window keeps the details and log location visible; use **Copy details** when
reporting a problem. Closing that window leaves Builder running in the tray.

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
- The default API is `https://switch.voipappz.io`; a local API
  (`voipappz-api`, `make dev`) can be chosen instead.

## Related repositories

| Repository | Role |
|---|---|
| `voipappz/app` | The React app Builder works on; the chat client (`src/builder-client/`) and the chat UI package (`packages/chat-ui`) |
| `voipappz/api` | The VoipAppz API; its `/api/mcp` tools reach the agent read-only; the vendored contracts |
