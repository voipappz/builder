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
  drive**; see "Get the app" below. A folder on WSL (`\\wsl.localhost\...`)
  or a network share does not work: `npm` on Windows cannot run there. To
  work on a WSL checkout, run the Linux Builder inside WSL instead (see "Run
  it" below); the Windows browser reaches it on `localhost`.
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed and
  signed in (run `claude` once in a terminal). The agent runs on your Claude
  account.
- A VoipAppz account on `https://switch.voipappz.io` to sign in to the
  previews. Node.js is not required: Builder installs the version the app needs.

**Get the app**

1. Install [Git for Windows](https://git-scm.com/download/win) if
   `git --version` does not work in PowerShell. The defaults are fine.
2. In PowerShell (not WSL):

   ```powershell
   cd $env:USERPROFILE
   git clone https://github.com/voipappz/app.git voipappz-app
   ```

   The repository is private: Git opens a browser to sign in to GitHub the
   first time. The checkout is `C:\Users\<you>\voipappz-app`.
3. Check that `voipappz-app\package.json` and
   `voipappz-app\src\builder-client\main.jsx` exist; onboarding looks for
   both. Builder makes its own worktree next to the checkout,
   `voipappz-app-builder`, on the branch `builder/work`.

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
6. When setup finishes, the Builder window opens: chat on one side,
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
- Tray icon dot: grey = not connected, yellow = setting up, green =
  running, red = setup failed. The setup window shows the seven setup
  steps as a checklist; **Setup details** below it has the full output.
- Tray menu: **Open VoipAppz Builder**, **Open in Browser**, **Connect App and API…**,
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
start, so your own checkout is never touched. Builder runs no Node itself: it
runs the app's own Makefile there, `make setup` the first time (Node through
mise, packages, login) and `make dev PORT=<port>`, which serves the chat and
the previews. When the API answers, it gives the agent its `/api/mcp` tools
read-only. The agent edits files without asking (its "Accept edits" mode) and
asks before running commands.

### With a voice bot

```text
cargo run -p builder-cli -- --app-repo ../app --start-app \
  --bot-dir ../pipecrab/examples/earshot-hebrew --open
```

A bot folder has a `builder.toml` with `kind = "bot"` and the same Makefile
contract (`make setup`, `make dev PORT=<port>`, `make test`, `make check`).
Builder runs it beside the app, waits for its health address, and the agent
changes the bot instead of the app; the Admin and User previews stay. The
page gets the bot's status, a restart (a Rust bot does not reload itself),
its log, and an audio WebSocket bridge that adds the bot's token from its
`.env`, so the page never holds it. The bot gets no API login.

In the desktop app, choose the folder under **3. Your voice bot** in
onboarding (tray: **Connect App, API and Bot…**); it is saved as `bot_dir`
in `builder.toml`. The tray's **Restart Voice Bot** rebuilds and restarts it,
and **View Voice Bot Log** opens its log.

Claude Code must be signed in (`claude`) on this machine. The app's login
gate reads `TEST_EMAIL`/`TEST_PASSWORD` from the environment or the
worktree's `.env`. Builder passes the API login typed in onboarding, or else
the one saved in the worktree's or the app checkout's `.env`. A cached
`.env.token` alone is not enough.

Before setup, Builder runs pre-flight checks and shows each result: the app
folder (a checkout with a Makefile, not on WSL or a share), `git`, `make`,
`bash` and `curl`, free disk space, the app and Builder ports, and whether
the API answers. A failed check stops setup with the reason.

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
