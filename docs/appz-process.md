# The appz process: from the mothership to a customer's app

How VoipAppz ships a custom app ("appz") per customer, and how Builder is the
place it gets built. Decided 2026-11-01 to 2026-11-03; status as of 2026-11-03.

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
| Builder: product folders, dev MCPs, Status-tab actions, desktop wording | builder main, **v0.1.17 released** | done |
| Connectix: `builder.toml`, `make setup` / `builder-stop` / `PORT`, `/voipappz/mcp` | connectix PR #2 | CI green, open |
| Connectix: conversations via live_react, `ChatSession` shared with the channel | connectix PR #3 | CI green, open |
| `@voipappz/conversations-ui` + chat-ui 0.2 | app PR #299 | CI green, open |
| Builder page: Product tab, action buttons | app PR #298 | CI green, open |
| API dev MCP at `/voipappz/mcp` | api #206 | merged |
| App dev MCP at `/voipappz/mcp`; Bot tab | app #295, #297, #285 | merged |
| appz template, `appz.toml`, registration, token exchange, appz → `/api/mcp` tools | — | not started |

## Roadmap

- **P0** Merge the four open PRs.
- **P1** The appz template repo (`appz.toml`, `builder.toml`, Kamal files) and an "Open appz" iframe tab in the admin, for one hand-made customer.
- **P2** Token exchange (API → Connectix) and registration rows; the bot row stores the appz URL.
- **P3** Connectix tools that call `/api/mcp` with the appz identity; node events into the appz.
- **P4** `appz new` in Builder; the first customer appz deployed from the Deploy button.

Reviews so far: Codex (bot core, research, Ruby-brain plan, Connectix as
brain, the Bot tab) and Claude code reviews of the Builder changes; all
findings fixed. A Codex review of everything above is pending (usage limit).
