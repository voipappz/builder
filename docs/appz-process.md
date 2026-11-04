# The appz process: from the mothership to a customer's app

How VoipAppz ships a custom app ("appz") per customer, and how Builder is the
place it gets built. Decided 2026-11-01 to 2026-11-04; status as of 2026-11-04.

## In three facts (2026-11-04)

1. **Each customer gets its own application** (mobile and desktop).
2. **Its screens are our existing React** (voipappz/app: phone, conversations
   and the rest), reused, not rewritten.
3. **Its logic is an Elixir app (Connectix) deployed per customer**: custom
   logic, agents, tools, flows. Elixir renders the React screens through
   [live_react](https://github.com/mrdotb/live_react).

```
Customer X's app (PWA, mobile and desktop)
  React screens from voipappz/app (pure components)
        │ live_react
Customer X's Elixir (Connectix, Kamal)   custom logic, agents
        │ API / MCP
VoipAppz mothership (voipappz-api + switch)   users, ACLs, calls, data

Builder: where X's Elixir is edited (chat, preview, tests, deploy)
```

Keep it simple: every proposal is checked against these three facts.

## The parts

| Part | Repo | Role |
|---|---|---|
| **VoipAppz core** (the mothership) | voipappz/voipappz-api + va-crystal | Tenants, identity, ACLs, telephony (FreeSWITCH + Kamailio nodes per region, DIDs, routes, queues), events, billing, the customer MCP at `/api/mcp`. The `bot` row is a pointer to a customer's appz |
| **The appz** | connectix-io/connectix | The bot brain, one container per customer: conversations, the Sagents agent, tools, approvals, human take-over. JSON-only API; no UI code of its own |
| **The UI** (one codebase) | voipappz/app | The React admin and user portal, and the pure components `@voipappz/conversations-ui` (on `@voipappz/chat-ui`) that the appz renders through live_react |
| **Builder** (the IDE) | voipappz/builder | Runs the app and the appz locally, gives the coding agent an MCP into each, and presses the appz's own test and deploy targets |

```
Customer admin (React portal) ── iframe tab ──▶ appz (Connectix: conversations-ui via live_react)
        │ VoipAppz JWT                              ▲ Connectix JWT (token exchange: planned)
   voipappz-api ◀── /api/mcp tools, appz identity (planned) ──┘
        │ NATS / ESL
   va-crystal node (FreeSWITCH) ◀── calls          Builder on the developer's PC runs it all
```

What stays out of scope for now: voice into the appz (mod_earshot), the
Chrome extension's Angular upgrade, an Angular MCP.

## Decisions (2026-11-03)

- **Builder is for end customers** who build a VoIP application (chat, bot,
  mobile app, CRM integration), not only for VoipAppz developers.
- **Each customer gets its own container, holding only the bot logic.** All
  data and permissions live in the mothership: no database inside an appz
  (Connectix's own conversation store is to move behind voipappz-api), and
  every ACL comes from voipappz-api.
- **Deploys go through the mothership,** with per-customer rights, approval
  and audit.
- **Shared screens are fixed in the React app repo** (`@voipappz/conversations-ui`).
- **Releases ship together** (app, Connectix, Builder). Voice comes later.

## Decisions (2026-11-04)

- **The customer app is React** (shell included). Connectix's Ionic/Angular
  phone retires over time; its Angular conversations screen is replaced by
  `@voipappz/conversations-ui`.
- **Screens are live_react pages:** Elixir renders each screen (LiveView);
  the React components stay pure (props in, callbacks out). One data layer.
- **PWA first,** served by the customer's Elixir; app-store wrappers later.
- **Builder edits only the Elixir code.** No local React app, worktree or
  `make dev` for it; the platform is `switch.voipappz.io`. Builder's own chat
  ships inside the installer (built from the app repo in Builder's release CI).
- **Milestone 1:** Builder works again (chat inside Builder), then one demo
  customer app: phone plus conversations, React through live_react, from its
  Elixir.
- Reference for "a customer app with tabs": Acrobits' web tab views
  (<https://acrobits.net/blog/use-cases/use-cases-web-tab-views-feature/>).

Answered the same day:

- **Phone:** the React app's sip.js phone (`PhoneScreen`, `useSipPhone`),
  registered straight to the switch. Elixir stays out of the media path.
- **Conversations data: the mothership's Postgres is the record**, and
  Connectix's Mnesia is only the live copy. Connectix `main` (`8f60640`,
  `Connectix.Mothership`, Broadway over gnat) sends every chat message
  (`chat.request.message.receive`) and call event
  (`chat.request.event.receive`) and reads transcripts back
  (`chat.request.event.list`). voipappz-api records them on queue group
  `connectix_workers` (branch `feat/connectix-nats-intake`, `db6980f`, not
  merged yet). DuckDB and the embedded llm_proxy are gone.
- **The appz's identity:** each payload carries the appz's account JWT
  (`MOTHERSHIP_ACCOUNT_TOKEN`) and `MOTHERSHIP_ENVIRONMENT_UUID`; the API
  checks that the environment belongs to that account.
- **End-user login:** through the mothership (`switch.voipappz.io`); the
  customer's Elixir keeps no users of its own. M1b mocks it behind one
  Elixir module (token in, user and ACLs out); the real one can reuse the
  same account-JWT check over NATS.
- **Demo customer:** a real VoipAppz user account on `switch.voipappz.io`
  (credentials are never stored in a repo or a doc).

Open: the NATS login design (subjects, token shape, where the Elixir side
lives); a customer-safe Builder (no code evaluation or SQL against the
mothership).

## What an appz is

One git folder, cloned from a template:

| File | Holds |
|---|---|
| `appz.toml` (planned) | persona, model, tools allow-list, skills, branding, the customer's `environment_uuid` |
| `builder.toml` | how Builder runs it: `kind = "product"`, health, the dev MCP path, `stop`, `requires`, `[[actions]]` (tests, deploy) |
| Connectix image pin, `config/deploy.yml`, `.kamal/` | Kamal deploy to that customer's host |
| `assets/vendor/*.tgz` | pinned `@voipappz/conversations-ui` and `@voipappz/chat-ui` (`scripts/ui-vendor.sh`) |
| custom Elixir | the customer's own tools, skills and flows on Connectix's base |

Plus rows in voipappz-api that are not in git: the bot row (pointing at the
appz), a provider row (its SIP identity), routes and DIDs to it, and an app
identity for `/api/mcp`.

## The process, step by step

1. **Start from the template.** Clone the appz template into a folder named
   for the customer (`appz new <customer>` in Builder, planned; today: copy
   Connectix and edit `builder.toml`).
2. **Open it in Builder.** Onboarding, section 3: choose the folder. Builder
   checks `requires` (Docker), runs `make setup` once, then `make dev
   PORT=<port>`, and waits for `/health/alive`. The Product tab shows the
   appz's page; the Status tab has its log, restart and action buttons.
3. **Build by chatting.** The agent edits the folder (Elixir for logic,
   `appz.toml` for persona and tools). It has four MCPs: `voipappz-product`
   (the appz's dev MCP: logs, code locations, docs, eval), `voipappz-app`
   (the React dev server's), `voipappz-api-dev` (a local API's) and the API's
   `/api/mcp` (read-only platform data). The UI is never edited here; screens
   come from `@voipappz/conversations-ui`.
4. **Test.** Status tab → **Run tests** runs the folder's `make test`. CI runs
   the same on the PR.
5. **Ship.** Merge; the Image workflow builds `ghcr.io/connectix-io/connectix:<sha>`;
   Status tab → **Deploy** (asks first) runs `make deploy` to that customer's
   host with Kamal. `make rollback VERSION=<sha>` undoes it.
6. **Register with the mothership** (planned, P2): on first deploy, create the
   bot, provider and route rows and store the appz's URL on the bot row.
7. **Use it.** The customer's admin opens the appz as a tab in the VoipAppz
   portal (iframe, token exchange planned); calls reach the appz through the
   node's events; the appz acts on the platform through `/api/mcp` (planned).

## Rules Builder enforces

- The agent writes only inside the folder it was given; API tools are
  read-only; it never sees logins or keys (filtered from its environment and
  from WSL's).
- Every `/bot/*` route (status, restart, log, actions, audio bridge) is
  behind Builder's launch secret, Host and Origin checks.
- An action marked `confirm` (deploy) is refused unless the page confirmed it.
- The folder's `stop` target runs on quit, when a start fails after the appz
  came up, and once before the first start, with the same `PORT`, so a
  detached container never outlives Builder.
- No "tidewave" naming in our code; the dev MCPs are `voipappz-*` at
  `/voipappz/mcp` (the npm/hex package names stay).

## Status

| Piece | Where | State |
|---|---|---|
| Builder: product folders, dev MCPs, Status-tab actions, desktop wording | builder v0.1.17 | released |
| Builder: actions in the background; unused upstream code removed (~7,000 lines) | builder **v0.1.18** | released |
| Connectix: `builder.toml`, `make setup` / `builder-stop` / `PORT`, `/voipappz/mcp` | connectix #2 | merged |
| Connectix: conversations via live_react, `ChatSession` shared with the channel | connectix #3 | merged; v0.1.25 tag pending |
| `@voipappz/conversations-ui` + chat-ui 0.2 | app #299 | merged |
| Builder page: Product tab, action buttons; Status tab follows actions | app #298, #300 | merged |
| Builder chat inside the installer; appz-only mode; onboarding = sign in with your VoipAppz account, then the appz folder | builder **v0.1.19** (#1), app #301 | released (verified on Linux end to end; not yet run on Windows) |
| API dev MCP at `/voipappz/mcp` | api #206 | merged |
| App dev MCP at `/voipappz/mcp`; Bot tab | app #295, #297, #285 | merged |
| appz template, `appz.toml`, registration, token exchange, appz → `/api/mcp` tools | — | not started |

## Roadmap

- **M1a — Builder works again.** Chat shipped inside Builder; the agent's
  folder is the Connectix appz only; MCPs: `voipappz-product` and
  `switch.voipappz.io/api/mcp`; onboarding: agent, appz folder, login.
- **M1b — One demo customer app.** A PWA served by that customer's Elixir:
  phone plus conversations, React screens through live_react. Fill the
  `conversations-ui` gaps the Ionic chat covers (thinking and notice
  messages, debug, wake, offline, prompts, Hebrew/RTL, narrow layout), then
  delete the Angular chat.
- **M2** The appz template (`appz.toml`, `builder.toml`, Kamal), login through
  the mothership, registration rows.
- **M3** More screens as tabs (configured per customer); Connectix tools on
  `/api/mcp` with the appz identity.
- **Later** A phone ↔ tab bridge (dial, call state) for our own tabs; app-store
  wrappers; customer tabs built in Builder; conversations data in the mothership.

Reviews so far: Codex (bot core, research, Ruby-brain plan, Connectix as
brain, the Bot tab) and Claude code reviews of the Builder changes; all
findings fixed. A Codex review of everything above is pending (usage limit).
