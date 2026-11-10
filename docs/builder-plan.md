# Builder plan: what Builder itself builds next

Companion to `docs/appz-plan.md`. That document says what a customer's
application is. This one says what Builder must do so that **a customer
builds their own application using Builder**, in the order it ships.

## Where Builder stands (v0.1.21, 2026-11-09)

- Signs in with the VoipAppz account; opens a local folder with a
  `builder.toml` (`kind = "bot"` or `"product"`): runs `make setup` once,
  `make dev PORT=…`, waits for health, restarts after edits, runs
  `[[actions]]`; chat on the left (built from voipappz/app, embedded), the
  folder's page on the right, a Status tab, and a Bot tab whose hold-to-talk
  speaks mod_earshot's wire protocol through Builder's bridge.
- Agent: Claude Code over ACP, writes only in the folder, VoipAppz API tools
  read-only, logins filtered from its environment.
- Installers: Windows exe, Linux .deb and AppImage, published on `v*` tags.

## The target loop

```
install Builder → sign in → New appz → the agent builds (Elixir in the folder)
   → push → the cloud dev instance reloads → see the page → talk to the bot
   → Deploy (mothership promotes dev to prod) → route a number → call it
```

## Releases

| Version | Builder feature | Needs outside Builder | Proof |
|---|---|---|---|
| **v0.1.22** | **Remote appz** (PR #9): `builder.toml url = https://…` opens a folder whose appz runs elsewhere; no make targets, health and page and the Bot socket at the url, `wss://` bridge | Nothing: the cloud Connectix at `cloud.voipappz.io:5100` already answers | Open a folder pointing at the cloud instance; the Bot tab talks to it |
| v0.1.22 | **Appz UX**: one pane titled from `builder.toml` (Admin/User tabs gone), suggestion chips in appz words, startup and onboarding wording as one story, setup progress in the main window | voipappz/app PR (`src/builder-client`), `client.ref` bump | The first screen a customer sees reads as "your appz", not "the app" |
| v0.1.23 | **Push to the dev instance**: Run = commit and push the folder to the customer's repo; Builder shows the instance's reload result and compile errors in the Status tab | Connectix extension loader (appz plan step 6); a dev instance per customer | Save → page updated in under five seconds |
| v0.1.23 | **Token from the mothership**: the Bot bridge and the page get the instance's token through the signed-in account, not from a local `.env` | Mothership endpoint: appz of this customer, its url and token | A customer never sees a token |
| v0.1.24 | **New appz**: clone the template from GitHub, name it, register it with the mothership, open it; the template ships three samples (a live screen, a custom call flow, a report) and an AGENTS.md | Template repo; mothership "create appz" (provisioning of the dev instance) | A fresh account reaches a running appz in minutes |
| v0.1.23 | **Deploy tab, the full tool** (see below): environments, deploy with confirm, status, versions and rollback, logs, number routing | Today: the folder's own make targets (Connectix has deploy, deploy-status, rollback, deploy-logs, deploy-version via Kamal). Later: the same through the mothership's deploy API | A customer deploys, watches, and rolls back without a terminal |
| v0.1.25 | **Agent guard rails and reference**: the catalog README and the appz contract as the agent's reference; refuse edits outside the customer folder; the agent login choice (bring your own or bundled) | Decision on the agent's login and pricing | The agent proposes Elixir and catalog widgets, never React |
| later | Status: live call log from the instance; evals for the bot from the Bot tab; Windows without WSL (nothing to run locally in the cloud model, so this falls out) | Instance events feed | |

## The Deploy tab: Builder provides the whole deploy tool

A customer never opens a terminal or a cloud console. Everything about
running their appz is in Builder, in one tab, and every outward step asks
first.

| Control | What it does | Today (folder make targets, via `[[actions]]`) | Target (mothership deploy API) |
|---|---|---|---|
| Environments | dev and prod, which one is selected, its URL and health | `url` + health of the remote appz | The mothership lists the customer's instances |
| Deploy | Build and ship the current folder to the selected environment; shows the build log live; confirm first | `make deploy DEST=…` (Kamal, exists in Connectix) | "deploy appz for customer C to prod" — the fleet runs Kamal; the customer holds no registry, host or SSH credentials |
| Status | What runs where: version, started at, health, containers kept | `make deploy-status` | Mothership status endpoint |
| Versions and rollback | The versions kept on the server; one click rolls back, with confirm | `make rollback` / `make deploy-version` | Mothership rollback endpoint |
| Logs | The running instance's log, tailing | `make deploy-logs` | Instance log feed through the mothership |
| Numbers | Which phone numbers reach this appz; route a number to dev or prod | Read-only today (API tools) | Mothership route endpoint, confirm first |
| Secrets | Names the appz needs (from `.env.example`) and whether each is set on the environment; values never shown | Not in Builder yet | Mothership secrets endpoint, write-only |
| Promote | Dev → prod: deploy the version that runs on dev, without rebuilding | — | Mothership promote endpoint |

Rules: one action at a time per folder (exists); a dropped request never
kills a run (exists); every control that changes something outward asks
first and acts only inside the signed-in customer; outputs are shown
without secrets.

## Rules that hold across releases

- The agent edits only the customer folder; API tools stay read-only;
  logins never reach the agent's environment.
- Every outward action (push to a customer repo, deploy, provisioning)
  asks first, and acts only inside the signed-in customer.
- Builder needs no toolchain on the customer's machine once the appz is
  remote: no Docker, no WSL, no Elixir. The local `make dev` path stays for
  VoipAppz engineers and the Noa-style bots.
- Windows code is compiled only by Installers CI; run it on the branch
  before merging desktop changes.

## Risks

| Risk | Hedge |
|---|---|
| The cloud page refuses to be framed (CSP or X-Frame-Options on the instance) | Check on v0.1.22's live test; the instance's endpoint allows Builder's origins (`ClientOrigins` exists in Connectix) |
| Push-to-see depends on the extension loader, which is engine work | v0.1.22 ships without it; the remote appz alone already gives "see and talk" against the cloud |
| Token in a local `.env` is a stopgap | v0.1.23 moves it behind the mothership |
| Builder's chat changes look when `chat-ui` is ported to shadcn | Port at dashboard-ui time with screenshots (Codex) |
