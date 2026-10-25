# VoipAppz Builder plan

Status: first version built and released (0.1.0, 2026-10-23). See "Status"
below for what works and [docs/how-it-works.md](docs/how-it-works.md) for how
it fits together. The rest of this file is the plan and its decisions.

## Status (2026-10-23)

| Area | State |
|---|---|
| Local server (`core/`, `cli/`) | Done. Stub page, launch secret, Host/Origin checks, only the page, websocket and `/mcp/api` routes |
| Chat (`../app` `src/builder-client/`) | Done, merged (app pull request 263). `ChatView` over ACP, permission bar, agent modes, Admin/User previews |
| Agent | Claude Code through `@agentclientprotocol/claude-agent-acp`, on the user's own account, chosen by the server |
| Where it works | Builder's own worktree of `../app` (`../app-builder`, branch `builder/work` from `origin/main`), created on first start |
| Write limits | The agent writes only in the app worktree: the server refuses edits outside it, in `.git` or through symlinks. Edits inside are accepted ("Accept edits" mode); commands still ask |
| API | Read-only, through Builder's `/mcp/api` proxy: write tools hidden and refused, login kept in Builder |
| Desktop app and installers | Done. Tray app (`desktop/`); `.exe`, `.dmg`, `.deb`, `.AppImage` built by `.github/workflows/installers.yml`; release v0.1.0 (unsigned pre-release) |
| Linux per-user install | Done. `install/install.sh` and `voipappz-builder-launch` |
| Contracts | Re-scoped to ACP, merged in the API (pull requests 140 and 152) |
| Not yet | A full chat turn (the account's Claude quota), React runtime tools, `/dev/mcp` read tools, changes/checkpoint panel, hosted `/builder/client.js`, signing and updates. WSL mode for the Windows app: a checkout under `\\wsl.localhost` is refused at onboarding (2026-10-25); running git, npm, the dev server and the agent through `wsl.exe -d <distro> --cd <path>` would need the write-boundary check to map Linux paths to the UNC root |

Decisions recorded 2026-10-17:

- **The chat is fetched from `../app` (2026-10-22).** The app
  builds its chat into a hosted chat client script and serves it with its own
  deployment. Builder's local server returns a stub page that loads that
  script from the app's deployment. Builder contains
  no React and no chat code; chat changes ship with app deploys.
- Builder has **no connection to the existing Assistant, VML or Bot**. The
  package holds presentation only. Builder does not call, import, wrap, extend
  or share storage with the Assistant hook, `/api/vmls/*`, `Mediators::Mcp::Chat`,
  any `Mediators::Bot::*` class, Ragents, the provider dialog, VML sessions or
  Bot runtimes. Builder's chat logic, API, model adapter and storage are new.
  Builder work also does not modify VML or Bot code; their changes, if any,
  are separate tasks.
- **All tools through MCP (2026-10-22).** The chat client
  drives a coding agent over ACP (Agent Client Protocol) through Builder's
  local server. The agent is the only MCP client and gets every tool from MCP
  servers: the Ruby API, the React project and the local workspace. No custom
  Builder REST API and no agent loop of our own. This replaces the earlier
  "Ruby MCP client" direction. See "Architecture: hosted chat, ACP agent, MCP tools".
- **Chat UI package status:** extracted and merged in the app (pull request
  241, 2026-10-18). The app uses it to build the hosted chat client. Builder
  no longer installs it, so publishing to GitHub Packages is optional.
- ~~Builder is a fresh codebase.~~ **Reversed 2026-10-23: Builder starts
  from an Apache-2.0 upstream desktop app.** Its Rust core and CLI are
  restored, renamed to VoipAppz Builder and hardened:
  per-launch secret on every request, no open `/proxy`, agent and shell
  commands only from Builder's own list and only inside the workspace root.
  Apache-2.0: keep the license and add a NOTICE.
  [docs/upstream-reference.md](docs/upstream-reference.md) lists the fixes.

Decisions recorded 2026-10-23:

- **Builder is the "chat builder"** of the app repo's customer app builder
  plan (`docs/customer-platform-lego-research-plan.md`, commit `414f4e9`).
  Builder's chat (the app's chat UI, ACP agent, MCP tools) replaces that
  plan's chat section; its start point on the old AI chat and
  `/api/vmls/generate` is dropped. Blocks and `app.layout.json` become what
  the agent edits. That doc needs a matching update in the app repo.
- **First users are VoipAppz developers**, working on the app and API source
  with the API running locally. The runtime tools (`/dev/mcp` in the API,
  the React MCP in the app) come before customer onboarding, installer and
  hosted isolation.
- **The code Builder works on is `../app`.** The first project is the
  VoipAppz React app itself: we use Builder to build our own version of it,
  and customer projects copied from it come later. The agent edits `../app`
  source in an isolated git worktree of `../app` (the repo already works
  in `app-*` worktrees), never in the main checkout, which carries other
  in-progress work.
- **API: the local API first.** `voipappz-api` under Docker Compose, at
  `http://127.0.0.1:<VA_WEB_PORT>` (5055 on the current machine, read from
  `voipappz-api/.env` as `make claude` does; never the `:5000` default, which
  is a different app). Builder checks `/health` before starting, as
  `make claude` does. `switch.voipappz.io` is a later, explicit choice.
- **Sign-in: as `make claude` does today.** The local tenant created by
  `make tenant`, sent as Basic auth to `/api/mcp`. The login is checked
  against `/api/devices` first (`MCP_AUTH=1 bin/dev-status.sh`) and kept in
  the OS credential store, never in the descriptor.
- **Preview and agent use the same API.** For Builder runs, the `../app`
  worktree's Vite proxy (`VITE_API_BASE_URL`) points at the same local API
  as the agent's tools. `../app/.env` currently points at
  `https://switch.voipappz.io`; Builder sets the worktree's own `.env` and
  never edits the main checkout's.
- **Previews of the app next to the chat.** The chat
  page shows the `../app` worktree's Vite dev server in frames beside the
  chat. Each edit the agent makes appears through Vite hot reload, with no
  rebuild. There are two previews, Admin (account login) and User (portal
  login, `UserAuthProvider`). The app keeps its login in `localStorage`
  (key `auth`, per origin), so each preview gets its own origin, for example
  `admin.localhost:3000` and `user.localhost:3000` (browsers treat
  `*.localhost` as secure). That keeps the two
  logins separate and stops the User preview from reusing the Admin login.
  The agent's browser tools act on these previews through the core's
  browser bridge.
- **Deploy with Kamal.** `../app` already deploys with Kamal
  (`config/deploy.yml`, image `nirlevi/voipappz-admin`, nginx on port 80),
  so the hosted chat client (`/builder/client.js`) ships in that same image
  and deploy, with no new service. The deploy host is still the
  placeholder `dest-required.invalid` and is chosen per destination. Kamal is
  also the option for any later Builder-side server (for example, a hosted
  workspace).
- **First agent: Claude Code on the user's own Claude account**, through its
  ACP adapter. Codex and a VoipAppz-paid model are later options. The Claude
  ACP adapter is a Node package; developers already have Node.

## Next milestones (planned 2026-10-25)

Decisions recorded 2026-10-25:

- **Web only, one codebase.** Builder edits `voipappz/app` only. The app
  is already designed for phones (responsive MUI layouts) and is used in
  the browser there; no native app is built. No Capacitor or Ionic shell,
  no Kotlin Multiplatform, no React Native: each would add a second
  codebase or build the agent cannot preview with HMR in a browser.
  `voipappz/mobile` (an empty Capacitor start) is not part of the plan.
- **Phone-width preview in Builder.** A third preview frame shows the user
  portal at phone size from the same dev server, so "on mobile" requests
  can be checked in the chat loop. It is a browser frame, nothing more.
- **Deploy through CI, not from the customer's machine.** Builder commits
  and pushes `builder/work`; a GitHub Actions workflow builds the image and
  runs `kamal deploy` to a destination. Local Kamal (Ruby, Docker, SSH) is
  a developer option only.
- **The model is swappable at the agent, not in the chat.** The server
  picks `agent_command`; a second ACP agent gives local or other models
  without touching the chat client.

| Step | Deliverable | Acceptance check |
|---|---|---|
| M1. Local stack test loop | Builder (Windows or WSL) against `voipappz-api` `make dev` + `make tenant` at `http://localhost:<VA_WEB_PORT>`, with a voice node from `voipappz/installer` registered to it | One chat turn reads real tenant data through `/api/mcp` and edits a screen that shows it; nothing touches `switch.voipappz.io` |
| M2. Phone-width preview | `core/src/builder.rs` adds `previews.mobile` (`mobile.localhost:<port>`, its own origin and login, same dev server); the app's `src/builder-client/BuilderApp.jsx` renders it in a 390×844 frame with a rotate control | "Make the call history easier to read on mobile" shows its result in the phone frame without a rebuild; Admin and User frames are unchanged |
| M3. Deploy from chat | `config/deploy.yml` gets a real staging destination; `.github/workflows/deploy.yml` with a `destination` input runs `kamal deploy`; Builder's **Deploy** action commits, pushes, triggers the workflow and shows its steps in the setup-style checklist with the resulting URL and commit | A change made in chat is live on the staging URL after one click; the page names the deployed commit; a failed deploy shows the workflow's error, never a stale URL as live |
| M4. Second agent, local model | Settings `agent_command` gains a picker: Claude Code (default), Codex (ACP adapter), Goose with Ollama; a one-day conformance check of each adapter against the chat client (permission requests, `tool_call` updates, cancel, session reload) | The same request completes on Claude and on one local model with the diffs reviewed side by side; an adapter that fails the conformance check is not offered |
| M5. WSL mode for the Windows app | A checkout under `\\wsl.localhost\<distro>` is offered as "Run in WSL": git, npm, the dev server and the agent run through `wsl.exe -d <distro> --cd <path>`; settings keep both path forms; `writable_in` maps Linux paths to the UNC root | Setup, a chat edit and HMR work with the checkout inside WSL; an edit outside the mapped root is refused as on Windows paths |

Order: M1, then M2 and M3 (both small, both visible), then M4, M5.
M1 is also where `/dev/mcp` (above) is first testable. M4 decides whether a
VoipAppz-hosted model (vLLM behind Kamal) is worth building.

## Existing work this plan builds on (surveyed 2026-10-23)

| Where | What exists | Effect on this plan |
|---|---|---|
| `builder` `core/`, `cli/` | The upstream desktop app's local server and CLI, restored and renamed | Builder's server is added next to it (`core/src/builder.rs`) |
| API `/api/mcp`, `docs/MCP.md` | Working MCP server: `Endpoints::Mcp` and `Endpoints::McpTools`, about 14 tools in `Mediators::Mcp::Toolbox` (nodes, devices, calls, logs, services, providers), specs, Claude Code wiring through `.mcp.json` and `make claude` | Builder's API MCP is this server. `docs/MCP.md` must be updated in the same change as any MCP change (API CLAUDE.md policy) |
| API `docs/carrier-number-plan` branch | Planned `numbers.search` and `numbers.buy` tools | More platform tools coming on the same toolbox |
| API `Mediators::Mcp::Chat` | The in-app Assistant runs the same Toolbox through `/api/vmls/generate` mode=mcp | Any tool added to the Toolbox reaches the Assistant; runtime dev tools go elsewhere |
| App `packages/chat-ui` (pull request 241) | `ChatView` and parts, controlled props, boundary test; the app's `AIChat` uses it | This is the chat Builder shows |
| App API Docs "MCP console" (`McpConsole.jsx`, `mcpApi.js`) | Tool picker fed by `tools/list` | A manual way to check new tools |
| App `docs/customer-platform-lego-research-plan.md` (commit `414f4e9`, only in the detached `app-lego` worktree, on no branch) | Customer app from blocks, `app.layout.json`, chat builder: v1 layout edits, v2 code edits, starting from the existing AI chat and `/api/vmls/generate` | Builder is this plan's chat builder (decided 2026-10-23); its `/api/vmls` start point is dropped |
| API `CUSTOMER_APPLICATION_SKELETON_PLAN.md`, `REACT_MUI_SKELETON_IMPLEMENTATION_PLAN.md`, `MINIMAL_CRM_PORTAL_IMPLEMENTATION_PLAN.md` | The customer starter and first CRM app | Decide what a Builder project's template is |

## Product direction

VoipAppz Builder is a chat-driven coding environment for custom PBX development
on the current VoipAppz system. A customer describes a change, the dedicated
Builder Assistant reads and edits their project, validates it, and shows the
running result in separate Admin and User previews. The customer refines the
result through chat and reviews the source diff before saving a revision.

The Assistant exists only in Builder. Remove its launchers and chat mounts
from `../app`. The app keeps the chat UI and serves it as the hosted chat
client that Builder loads. The chat drives a coding agent over ACP, and the
agent gets its tools only from MCP servers. None of it has a connection to the
Assistant, Bot or VML code, endpoints, storage or libraries. Bots remain composed of VML, run through their own
runtime, and are left unchanged by Builder work.

The first API target is `https://switch.voipappz.io`. Its authenticated MCP
provides tenant-scoped platform tools. The React/Vite app provides the customer
project and Admin/User previews. The upstream desktop app was studied; its useful
local-server, workspace and process patterns are recorded in the reference doc
and re-implemented in Builder's own code. Builder follows the same
architecture: a hosted chat client loaded by a local server, an ACP agent, and
MCP tools. The chat client, the MCP servers and the agent supervision belong
to VoipAppz; the model runs inside the agent.

## What the customer gets

Open **VoipAppz Builder**, connect their VoipAppz API account, and select a
customer project based on the current React/Vite app. In the Assistant, which
uses the same chat UI the admin app ships as a package, ask:

> Add an extension search field to my PBX admin screen, and make the user call
> history easier to read on mobile.

The dedicated API Assistant reads the actual source and MCP context, edits the
customer checkout, checks the change, and refreshes Admin and User previews.
The customer can ask for a refinement or undo, then inspect the code diff and
save a validated revision. Shipping that revision to a live customer deployment
is the next milestone; the first milestone must not label a saved draft as live.

This is a coding loop. Completing it does not require a customer to copy code
from chat, install an agent by hand, or issue shell commands manually. Builder
downloads, launches and supervises the agent itself.

## Repository ownership

| Repository | Changes in this milestone |
|---|---|
| `builder` | Fresh binary and local server: stub page that loads the app's hosted chat client, ACP proxy that launches and keeps the agent alive, workspace MCP server (files, search, patch, commands, checkpoints), browser-tool bridge, installer, Vite supervision, preview windows. No React, no chat code |
| `../voipappz-api` | Builder tools added to the existing MCP server at `/api/mcp` under a builder namespace, scoped by the caller's token; Builder sign-in for the desktop app; no agent loop, no Builder REST endpoints |
| `../app` | Build and serve the hosted chat client (from `@voipappz/chat-ui`, speaking ACP to Builder's local server), remove Assistant launchers/mounts, host the React MCP server (dev-only Vite plugin plus browser bridge); provide PBX source and Admin/User preview routes; existing logo/theme remain authoritative |

The agent owns the conversation and the model calls. Every tool it uses comes
from an MCP server. The API serves platform and project tools over MCP and
authenticates the caller. The app serves the chat client.
Bot and VML code, endpoints, storage and libraries are not used or modified
by Builder.

## Delivery order

Each row is a reviewable implementation increment. UI work can use recorded
contract fixtures after step 1, but fixture output is never presented as a live
coding result. Advance only when the acceptance check for the step passes.

| Step | Deliverable | Acceptance check |
|---|---|---|
| 1. Project and connection contract | Versioned project descriptor; API session/run/event schemas; connection states | API, worker and UI agree on identity, revisions, cancellation and errors; no secrets in descriptor |
| 2. Branded connection shell | Builder binary whose local server serves the stub page and loads the app's hosted chat client; per-launch secret; API login and capability check; project selection | Opens using our UI assets, connects to the intended API/customer, shows useful failures; local server rejects requests without the launch secret |
| 3. Inspect a real project | Isolated checkout and source list/search/read/diff tools | Assistant can identify the relevant existing components; path/symlink escape and cross-project access fail |
| 4. Agent over ACP with MCP tools | ACP proxy launches the agent with the workspace, React and API MCP servers; builder namespace tools on `/api/mcp` | A real chat turn reads source through MCP and reports actual results; the agent's tool list is exactly what the MCP servers listed; no Assistant, VML, Bot or Ragents reference |
| 5. Edit and validate | Revision-checked patches, bounded command execution, checkpoints, cancellation and revert | Agent changes one component; validation result belongs to that revision; stale writes fail; cancel stops tasks |
| 6. Vite and two previews | React MCP server reachable by the agent, browser tools through Builder's bridge, HMR, separately authenticated Admin/User preview origins | The agent lists and calls React tools; both previews reflect the same revision; User cannot gain Admin rights; failed Vite startup is visible |
| 7. Conversation and review | The app's hosted chat client, loaded by Builder, speaks ACP and streams text, tool calls, changes and validation; session reload after reconnect; app chat mounts removed | Two real turns refine the same project; reconnect reloads the session without repeating tool calls; exact diff/checkpoint can be saved; app has no Assistant entry point; chat package has no app auth/API imports |
| 8. Installer smoke proof | One development installer for the selected first OS, clean launch and reconnect | Fresh install -> sign in -> select project -> chat edit -> previews -> restart -> same saved project |

Step 8 delivers a development installer. Public release requires signing/update
configuration, platform-specific smoke tests and the publication work below.
The initial OS is not chosen yet; keep Windows NSIS, macOS and Linux packaging
tasks visible rather than claiming all are validated by one smoke test.

## Starter contract to implement first

### Shared chat UI package and API independence

See the [source dependency audit](docs/assistant-dependency-audit.md) for the current
call graph, exact coupling and the tests required to prove separation.

The chat UI stays in `../app`. Builder fetches it at runtime as the app's
hosted chat client (see "How Builder loads the chat"). This is neither an
iframe of the old Assistant nor a copy of its source. Current state of the
source (2026-10-17):

| File | Lines | Role today |
|---|---|---|
| `src/components/AIChat/AIChat.jsx` | 737 | Message list, tool-call chips, composer, session sidebar; imports `ProviderDialog` and `providersApi` directly |
| `src/components/AIChat/useAIChat.js` | 663 | Sessions and NDJSON run-event streaming against `/api/vmls/*`; imports `useAuth`, `useUserAuth`, `config`, `providersApi` |
| `src/components/AIChat/aiMockService.js` | 311 | Mock sessions and stream for tests |
| `src/components/AIChat/useAIChat.test.js` | 299 | Hook tests |
| `src/context/AIChatSidebarContext.jsx` | 33 | Drawer/sidebar open state |

Package work in `../app`:

1. Create a workspace package `packages/chat-ui` (proposed name `@voipappz/chat-ui`)
   with React, MUI and MUI icons as peer dependencies. Move in only the
   presentational components from `AIChat.jsx`: message list and bubbles,
   tool-call chip, composer with send/stop, session list, copy button and the
   CSS. Document the CSS variables it reads (`--theme-*`) as the theme contract
   that both app and Builder provide.
2. The components are controlled. They receive messages, tool calls, sessions
   and busy state as props and report user actions through callbacks
   (`onSend`, `onStop`, `onSelectSession`, `onNewSession`, `onDeleteSession`).
   The package defines its own neutral message and tool-call prop shapes. It
   contains no fetch, no stream parser, no session storage and no auth.
3. `useAIChat`, `aiMockService`, `AIChatSidebarContext`, `ProviderDialog` and
   `providersApi` do not move into the package. The package must not import
   anything from the app's `context`, `services`, `config` or `Providers`.
4. Add storybook-style fixtures or plain prop fixtures and component tests in
   the package so it is testable without any backend.
5. Build a hosted chat client entry in the app from the package (see "How
   Builder loads the chat"). Publishing the package to a registry is optional,
   since Builder does not install it.
6. Remove the app's mounts listed below. The app keeps building the package even
   though it no longer renders the Assistant.

Builder owns everything behind the components: its own chat hook, its own
event types designed for coding runs, its own client for the builder API and
the local worker stream. Nothing in Builder calls, imports or mirrors the
Assistant's endpoints, event names or session keys. A dependency check in CI
fails if the package or Builder references `useAIChat`, `/api/vmls`,
`AIChatSidebarContext`, `providersApi` or any Bot/VML module.

Source audit found these actual connections to remove from the app:

| Current connection | Required implementation change |
|---|---|
| App TopBar `shell.assistant` button -> `openResourceFinder()` | Remove the Assistant launcher; retain unrelated screen search |
| `CommandPalette.jsx` lazy-loads `AIChat`; Ctrl/Cmd-K opens it | Remove the chat mount/shortcut; preserve ordinary navigation search only if separately implemented |
| `Bots/Bots.jsx` imports and renders `AIChat` | Out of Builder scope: Bot screens are not touched. Decide separately whether Bots keeps the old chat; the package extraction must keep that import working or leave it on the app's own copy |
| `ApiDocs/McpAssistant.jsx`, `services/mcpAssistant.js`, global-search question helpers | Remove Assistant chat surfaces/callers; preserve non-chat API docs and useful MCP inspection |
| `AIChatSidebarContext`, providers, translations, mocks and tests | Stay in the app, not in the package; delete them once nothing in the app uses them |
| `useAIChat` calls `/api/vmls/sessions` and `/api/vmls/generate` | Stays in the app with the old Assistant and is retired with it; Builder never uses it |
| `Mediators::Mcp::Chat` creates `Mediators::Bot::RagentsProvider` | Builder does not use either. It has its own model adapter under a builder namespace |
| VML endpoint stores `vml-index:*` and `vml-chat:*` | Builder has its own project/session storage and server-side ownership checks; it never reads or writes VML keys |

**Model adapter.** Builder gets its own adapter under a builder namespace in
`voipappz-api`. It does not use `Mediators::Bot::*`, Ragents or any Bot prompt,
tool, session, event or execution path. Whether it reads the customer's LLM
key from the generic `Provider` record is to be confirmed: that record is
customer configuration, not Bot code, but if it carries Bot-specific behavior
Builder stores its own model credentials instead.

**API.** The builder API is a new endpoint family. It does not extend, wrap,
call or fall back to the VML endpoint, the MCP-chat branch or any Bot endpoint.
Builder work does not change the VML or Bot endpoints; retiring the old
MCP-chat branch is a separate, non-Builder task. Tests and a dependency check
must prove builder code never references `Mediators::Bot`, `Mediators::Mcp::Chat`,
`Ragents`, `vml-` storage keys or VML/Bot endpoints, and never constructs a Bot
runtime.
Authorize builder sessions and worker grants server-side: hiding a button or
checking Origin is not an authorization boundary.

### How Builder loads the chat

Builder loads its chat from `../app`:

1. The app adds a chat client entry, proposed `src/builder-client/main.jsx`,
   built as its own script (proposed `/builder/client.js`) and served by the
   app's deployment next to the admin. It renders `@voipappz/chat-ui` and
   speaks ACP to Builder's local server.
2. Builder's local server answers `/` with a stub page: meta tags for the
   Builder version, OS, local ports, the selected project and API target, and
   a script tag for the client. Where the client comes from is a setting,
   the chat client address, with two standard choices:

   | Choice | Address | Use |
   |---|---|---|
   | Hosted | `https://switch.voipappz.io` | Default. The client deployed with the app for that environment. |
   | Local run | `http://localhost:3000` | A developer's own `../app` running locally, so chat changes show without a deploy. |

   Any other https origin, or plain http to localhost, may be entered. The
   address is set in this order: the `--client-url` flag, the
   `VOIPAPPZ_BUILDER_CLIENT_URL` variable, the project descriptor's optional
   `chatClient.url`, then the Hosted default. The shell shows the address in
   use. The descriptor field is in the v1 schema: an optional object with an
   https or localhost `url`, same rules as `api.baseUrl`.
3. The client reads the meta tags and connects back to the local server with
   the per-launch secret for ACP, file watching and the browser bridge.
4. Builder ships no React and no chat code. A chat change is an app deploy.

Rules: the stub page sets a strict Content-Security-Policy that allows
scripts only from the configured client origin; the local server checks the
per-launch secret on every request, not just Origin; when the client cannot
be fetched, the stub shows an offline page naming the URL it tried. The client must not read admin login tokens
from the app's storage; Builder sign-in is its own grant.

### Architecture: hosted chat, ACP agent, MCP tools

From the upstream source (`ws/acp.rs`, `ws/mcp.rs`):
the browser chat speaks ACP to a coding agent, the local app proxies that link
and keeps the agent alive, the chat tells the agent which MCP servers to use
when it opens a session, and the agent calls those servers itself. Browser
tools reach the agent through a reverse MCP bridge in the local app.

```text
App's hosted chat client (loaded by Builder's stub page)
   |  ACP: prompts, streamed text, tool calls, session reload
   v
Builder local server --ACP proxy--> Agent (Claude Code or Codex, launched by Builder)
                                       |  MCP client, the only one
            +--------------------------+---------------------------+
            v                          v                           v
  Workspace MCP (Builder)     React project MCP (app)      VoipAppz API MCP (/api/mcp)
  files, search, patch,       routes, components,          platform tools plus
  commands, checkpoints       build errors; browser        builder project tools,
                              tools via Builder bridge     caller-scoped token
```

| Part | Owner | Role |
|---|---|---|
| Chat client | `../app`, hosted script built from `@voipappz/chat-ui` | ACP client. Shows text, tool calls, diffs and validation. Passes the MCP server list when it opens a session. |
| Stub page and ACP proxy | `builder` local server | Serves the stub page. Downloads and launches the agent per project, buffers its notifications, reloads a session after the client reconnects, cancels on request. |
| Agent | Claude Code or Codex | Runs the conversation and the model calls under the customer's own agent account. It is the only MCP client. |
| Workspace MCP | `builder` local server | Tools on the isolated project copy: list, search, read, revision-checked patch, bounded commands, changes, checkpoint, revert. Enforces the workspace root. |
| React project MCP | `../app`, dev-only Vite plugin | Server-side project tools, plus browser tools for the Admin and User previews forwarded through Builder's bridge. |
| API MCP | `../voipappz-api`, existing `/api/mcp` | Existing platform tools plus a builder tool namespace, authorized by the token from Builder sign-in. |

Rules:

- **Only MCP for tools.** No Builder REST endpoints and no custom tool protocol.
- **Honest discovery.** The tool list the user sees is what each server's
  `tools/list` returned in this session. A disconnected server surfaces as a
  tool error, never a fabricated result.
- **Dev only.** The React MCP plugin registers only in `vite serve`; a
  production build contains no MCP server, and CI checks the bundle.
- **Authenticated locally.** The workspace MCP, React MCP, ACP proxy and
  browser bridge accept only the per-launch secret. The upstream local
  server had no auth; that mistake is not repeated.
- **No Assistant, VML or Bot code** in any of these parts.
- **Agent choice is open.** Which agent ships first, and whether a customer
  needs their own Claude or OpenAI account, is a product decision still to be
  made. Verify the Codex and Claude Code ACP adapters before choosing.

### Runtime MCP tools (planned 2026-10-23)

The agent gets tools that run inside the live app, served by the app itself
while it runs in development: one set in the Ruby API, one as a Vite plugin
in the React app, following existing open-source (Apache-2.0) runtime MCP
servers for Rails and for Vite. Builder needs both for the VoipAppz codebases. Grep and file reads cover source;
these tools cover what only the running app knows.

| Tool | Rails reference | Vite reference | VoipAppz home |
|---|---|---|---|
| `project_eval`: run code inside the running process, with stdout and a timeout | Ruby `eval` | JS in a worker | App only (the Vite dev server). Not in the API: the agent never writes there |
| `execute_sql_query`: parameterized, first 50 rows | ActiveRecord or Sequel | none | API: Sequel on the development database, read-only transaction |
| `get_logs`: tail plus a regex filter | `log/<env>.log` | server console | API: the local server log. App: Vite's console |
| `get_docs`: docs for the exact locked version | gems | packages and types | API: gems from `Gemfile.lock`. App: `node_modules` |
| `get_source_location`: where a symbol is defined, including generated methods | yes | yes | Both |
| `browser_eval`: drive the page in a real browser | through its hosted client | through its hosted client | Builder's browser bridge (Admin and User previews) |

**API: reuse the existing MCP server.** `Endpoints::Mcp` (public resources,
`/tasks/mcp`) and its subclass `Endpoints::McpTools` (`/api/mcp`, token
auth, `Mediators::Mcp::Toolbox`) already implement JSON-RPC, `initialize`,
`tools/list`, `tools/call` and MCP tool errors (`isError`). The runtime
tools reuse that code as a third subclass, `Endpoints::McpDev`, with its own
`Mediators::Mcp::DevToolbox`:

- Not in `Mediators::Mcp::Toolbox`. That toolbox is shared with the in-app
  Assistant (`/api/vmls/generate` mode=mcp), so a tool added there reaches the
  Assistant too. Eval and SQL must not.
- Mounted at its own path (proposed `/dev/mcp`) only when `RACK_ENV` is
  `development` and code reloading is on; the endpoint refuses to start
  otherwise, like the Rails reference. Localhost requests only. Never in the
  production image's routes, and a spec proves `/dev/mcp` is absent in
  production.
- The ported tool code follows the Rails reference (Apache-2.0): add a NOTICE
  entry for any copied file. The Railtie, toolbar and HTML injection are not
  ported, since the API is Pliny/Sinatra and has no pages.
- The existing `logs.search` tool (InfluxDB syslog) stays the platform log
  tool on `/api/mcp`. `get_logs` here reads only the local development log.

**App: the React project MCP is a dev-only Vite plugin.** A Vite plugin with
`apply: 'serve'`, so `vite build` never contains it, exposing the four core
tools on the dev server, ported into our own plugin at the descriptor's
`vite.mcpPath` (`/__voipappz/mcp`). Either way CI checks that the production
bundle has no MCP code.

**The chat is the app's chat UI.**
`packages/chat-ui` in `../app` already exports `ChatView` (messages, tool-call
chips, reasoning, sessions, composer, stop) with no data logic. The hosted
client entry wraps it with an ACP client and maps ACP updates onto its props:

| ACP `session/update` | `ChatView` prop |
|---|---|
| `agent_message_chunk` | assistant message `content`, appended |
| `agent_thought_chunk` | message `reasoning` |
| `tool_call`, `tool_call_update` | `toolCalls` `{ id, name, args, result, status }` |
| `plan` | reasoning steps |
| prompt turn running or finished | `busy`; Stop sends `session/cancel` |
| Builder's session list | `sessions`, `onSelectSession` (session load) |

Who uses which tools: a customer's agent works on the customer's React
project against the production API, so it gets the workspace MCP, the React
MCP and `/api/mcp`. `/dev/mcp` exists only where the API itself runs in
development, i.e. VoipAppz developers working on the API with Builder.

### Live API evidence

Read-only probes on 2026-10-16 established:

- `https://switch.voipappz.io/tasks/mcp` responds to MCP initialize and lists
  OpenAPI, integration Skill/readiness, errors and billing documentation resources.
- The public server reports protocol `2025-06-18`, version `1.1.0`, with resource
  capabilities. It is not the authenticated tool or chat service.
- `https://switch.voipappz.io/api/mcp` rejects an unauthenticated tools/list
  request with HTTP 401, "Missing Authorize token."

No authenticated tools were invoked and no live phone-system data was modified.
Successful authenticated tools/list and the new builder API deployment remain
implementation prerequisites. MCP supplies tools/context; it does not by itself
provide a streaming LLM conversation. Builder's chat connects to dedicated
builder API endpoints which use those tools.

[builder-project.example.json](examples/builder-project.example.json) is a
non-secret design fixture that no code accepts yet. Add a versioned parser and
validation before wiring it into startup. Its `vite.mcpPath` is the proposed
path of the React MCP server plugin.

Suggested records in the API:

- **Project:** authorized tenant, source/template revision, current draft revision,
  worker binding and preview environment. Local paths are worker-local values.
- **Session:** project, caller, conversation and active preview context. Separate
  from existing Bot/VML histories and the old generic chat session key.
- **Run:** session, base revision, current state, cancellation request, execution
  budget and validation revision. One writer per project at a time initially.
- **Checkpoint:** exact source revision, diff summary and validation evidence.

Proposed API operations (names are not implemented yet):

```text
POST /api/builder/projects
POST /api/builder/projects/:project_id/sessions
POST /api/builder/sessions/:session_id/runs
GET  /api/builder/runs/:run_id/events
POST /api/builder/runs/:run_id/cancel
GET  /api/builder/projects/:project_id/changes
POST /api/builder/projects/:project_id/checkpoints
```

Use the existing authenticated API request conventions after verifying their
actual contracts. Events carry sequence, project/session/run IDs, revision and
type. Stream `run.started`, text deltas, tool start/result, changes, validation,
permission requests, completion/cancellation/failure. Replay by sequence on
reconnect; tool execution uses separate idempotency keys so replay cannot repeat
an edit or command. Do not fabricate success events when a worker disconnects.

Initial workspace tool surface:

| Tool family | Required behavior |
|---|---|
| Source list/search/read | Relative paths, bounded output, binary/secret exclusions, canonical-root checks |
| Patch | Expected revision, atomic application, diff result; no unconstrained absolute paths |
| Validate/command | Workspace-scoped process, explicit execution grant, time/output limits, process-tree cancellation |
| Changes/checkpoint/revert | Exact revision identity; reject overwriting unrelated changes; retain an undo path |
| Preview status/logs | Actual URL, process health, selected role, bounded error output |
| Runtime MCP | React MCP tools called by the agent directly, browser tools through Builder's bridge; original argument schemas; honest tool discovery/errors |

Path checks are not a shell sandbox. Execute customer code in an isolated
workspace with bounded network access and no control-plane/model secrets. Local
development may run a trusted proof, but that does not satisfy hosted isolation.

## Installer and first-run setup

The installer connects services; it does not install or provision a production
VoipAppz API server as an incidental side effect.

Initial target: `https://switch.voipappz.io`, selected by the user. Display the
target explicitly and allow changing it. An API deployment without the new
builder endpoints must show "Builder API upgrade required"; do not silently
fall back to the old Assistant or any VML/Bot endpoint.

1. **Welcome:** VoipAppz branding from the app. Explain that the user is opening
   a coding workspace for their PBX.
2. **API connection:** enter/select the API base URL, verify reachability and
   builder capability/version, and sign in using the platform's supported flow.
   Distinguish unreachable server, invalid login, unauthorized project and older
   API missing builder support. Clearly show the configured production target.
3. **Project:** create from a pinned app template or select an existing checkout.
   Show source location and create an isolated draft/worktree without overwriting
   existing changes. Match the selected API environment to the app proxy config.
4. **Vite:** attach to a known dev server or start one in the managed workspace.
   Validate project identity as well as HTTP response; another process on port
   3000 is not a successful connection. Supervise Vite and display startup logs.
5. **MCP checks:** verify public docs, authorized API tools and the React MCP server via real
   initialize/list/call probes. Tools and permissions shown must be discovered,
   not inferred from a green TCP connection. Ruby dev MCP is optional and local
   or isolated-development-only, never required on the shared production API.
6. **Preview sessions:** establish separate Admin/User storage origins with
   short-lived authorized sessions. Confirm both see the same project revision.
7. **Ready:** open Assistant plus previews, show active API/project/environment,
   and provide one example coding request. Persist connection metadata; secrets
   belong in the OS credential store or server-managed worker grants.

On restart, validate saved connection metadata and renew credentials. On exit,
stop only processes the builder owns. An attached Vite or API process belongs to
its owner. Reconnect, logout and changing API invalidate old grants and previews.

For an installed customer experience, supply a tested managed Node runtime or
use a hosted worker; customers should not need Rust/Cargo to run a release.
Verify downloaded runtime/artifact checksums and signatures. Packaging must
distinguish installing the builder from installing project dependencies, which
can execute package scripts inside the isolated workspace.

## Origin of the local server

`core/` and `cli/` started from an Apache-2.0 desktop app's local server and
CLI (see `NOTICE`), renamed to VoipAppz Builder throughout.
[docs/upstream-reference.md](docs/upstream-reference.md) records the
mechanisms kept (process-group cancellation, optimistic writes, multiplexed
websocket, reconnect without replay, login-shell environment capture) and the
mistakes fixed (unauthenticated local server, open proxy, no workspace root,
secrets in logs). Builder's own server is `core/src/builder.rs`; the desktop
app is `desktop/`; installers are built by `.github/workflows/installers.yml`.

## VoipAppz branding checklist

Reuse existing assets, not an AI-generated replacement:

- `../app/public/images/VA_logo_blue.png`
- `../app/public/images/VA_logo_dark.png`
- `../app/public/images/VA_logo_white.png`
- `../app/src/theme/theme.js` and its shared theme tokens

`Login.jsx` and `Settings.jsx` show how the app selects the logo variants. During
implementation, copy versioned brand assets into builder packaging with recorded
source provenance; released installers must not depend on a sibling filesystem
path. Use the suitable mark for small tray/icons without stretching the wordmark.

| Surface | Planned change |
|---|---|
| Product/window/install name | VoipAppz Builder |
| Tauri application identity | Proposed `io.voipappz.builder`; verify org identifier convention before packaging |
| Logos and theme | Match app light/dark treatment and shared theme |
| App/tray/PWA icons | Generate required platform sizes from approved existing assets |
| Windows NSIS | Own installer icon, header/sidebar art and publisher/signing configuration |
| macOS/Linux | Own bundle/desktop metadata, icons and package naming |
| CLI/help/menu/errors | VoipAppz language and support URLs |
| Config/log/cache directories | Own VoipAppz namespace from the first commit |
| File association | Add only with handling code and tests; omit initially |
| Updater | Own release URL and public verification key; disabled until configured |
| Legal attribution | Keep `LICENSE` and `NOTICE` (Apache-2.0) with every source and installer |

Keep the no-secret-logging rule from the reference doc as a lint or
review check from the first configuration module onward.

## Implementation acceptance matrix

- Real end-to-end coding request and follow-up; inspect changed source, not just
  an Assistant reply. HMR and both roles render the resulting revision.
- Validation failure appears in chat and is repairable in the same session.
- Cancel, worker restart and browser reconnect do not leave orphaned commands
  or duplicate patches. A saved checkpoint survives worker replacement.
- Cross-tenant/project access, stale revision writes, traversal/symlink escapes
  and unexpected preview origins are rejected at the enforcing service.
- User preview cannot access Admin API operations or the builder's credentials.
- Public documentation MCP, tenant business MCP, Vite development MCP and optional
  Ruby development MCP retain their separate authority and tool namespaces.
- Installer tests cover missing runtime, unavailable/wrong API, expired session,
  occupied Vite port, wrong project, broken dependencies and clean restart.
- No public endpoint exposes unrestricted source/shell access; no development
  MCP is bundled into the customer's static production application.

API specs run in Compose `web`. App local verification is lint-only under its
current policy; builds/browser tests use CI or an explicit user exception.
Builder code uses focused unit tests and server integration tests (`cargo
test`, 152 tests). CI runs them in `.github/workflows/test.yml`; the desktop
app compiles only in `installers.yml`, since it needs WebKitGTK on Linux.

## Follow-on milestone: publish a customer application

Build an immutable artifact for the accepted revision, validate it against the
selected API, publish to a customer preview/release destination, and support
rollback to a known artifact. Define source export/Git sync and update ownership.
Code deployment and phone configuration/provisioning are separate operations;
restoring a frontend artifact cannot reverse a real call or arbitrary data change.

## First implementation task

Order of work:

1. Repository restart: done 2026-10-20.
2. Chat UI package in `../app`: done (pull request 241, 2026-10-18).
3. Step 1 contracts: written in `contracts/` with a Node validator, vendored
   into the API with a Ruby validator (API pull request 140), reviewed by
   Codex and fixed. Re-scoped to the ACP design on 2026-10-23: the
   descriptor (now with the optional `chatClient.url`), connection state,
   project, error and checkpoint stay; session, run, event and the run
   lifecycle were removed, since ACP defines sessions, turns and streamed
   updates. Merged in the API (pull requests 140 and 152).
4. Done 2026-10-23. Restore the upstream core into `builder`: the Rust
   workspace (`core`, `cli`, Tauri shell), renamed to VoipAppz, Apache-2.0
   license and NOTICE kept. Before anything else, harden it: per-launch
   secret on every request, remove `/proxy`, agent commands from Builder's
   own list (Claude Code ACP adapter first), shell and file routes only
   inside the workspace root, no config values in logs. Stub page points at
   the chat client address (default `https://switch.voipappz.io`, local
   `http://localhost:3000`). Run it as the CLI; the Tauri tray comes later.
   (The tray app and installers were added the same day.)
5. Done 2026-10-23, app pull request 263. In `../app`, add the hosted chat client entry: `ChatView` from
   `packages/chat-ui` wrapped in an ACP client that speaks the core's
   websocket, mapping ACP updates onto its props. Run it with `vite` on
   port 3000.
6. Open: blocked only by the Claude account's quota. Checked live so far: no
   secret gives 401, a foreign Origin 403, `/shell` 404; the agent starts in
   the workspace even when the client names `/etc` or its own command;
   `initialize` and `session/new` succeed. Proof: from the chat, Claude Code (user's own account) finds an existing
   component in `../app` and reports the file and line.
7. In `../voipappz-api`, add `/dev/mcp` (`Endpoints::McpDev`,
   `Mediators::Mcp::DevToolbox`) with read tools only, since the agent
   writes only in the app: `execute_sql_query` in a read-only transaction,
   `get_logs`, `get_source_location` and `get_docs`; no `project_eval`.
   Development and localhost only, and update `docs/MCP.md`. Builder hands
   it to the agent next to `/api/mcp`, through its read-only proxy.
8. In `../app`, add the React MCP (runtime tools) as a dev-only Vite
   plugin, and hand it to the agent.
9. Then edits, validation, checkpoints, previews and the browser bridge
   (delivery steps 5 to 7), the lego plan's blocks and `app.layout.json`,
   and only after that the installer and customer onboarding.
