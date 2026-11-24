# Plan: Builder with nothing to install

Written 2026-11-24. The owner wants to use Builder without installing it. A
draft: the decisions at the end are the owner's.

## What "no install" means

Builder opens as a web page (`https://builder.voipappz.io`, name open). You
sign in with your VoipAppz account, pick a platform (Web, Mobile, Chrome
extension), and get what the desktop app shows today: the chat on the left,
the running app on the right.

WebAssembly cannot do this: Builder's core starts programs (`make`, the
Android emulator, the coding agent), edits files and opens ports, which a
browser forbids. The core has to run somewhere. So "no install" means **the
core runs on a server, and the browser shows its page**.

## Why this is a small step, not a rewrite

Builder's core is already a web server (`core/src/builder.rs`): it serves the
chat page, carries the chat over a websocket to the agent, and frames the
project's page. The desktop app only starts that server on `127.0.0.1` and
opens a window on it. The phone's screen is already a web stream
(ws-scrcpy). What is local-only today:

| Today (one user, one PC) | Hosted needs |
|---|---|
| Listens on `127.0.0.1`, guarded by a per-launch secret in the URL | A real login (the VoipAppz account, as the setup screen already checks), HTTPS, sessions |
| One workspace, one project, one agent per process | One isolated workspace per user |
| The project page and the phone stream on local ports (4200, 5175, 8000), framed directly | Those ports reached through Builder's own address (a reverse proxy per session), since the browser cannot reach the server's localhost |
| The agent is Claude Code signed in on the user's PC | The agent signed in per user on the server (the user's own Claude login or an API key) |
| The emulator uses the PC's KVM | A server with KVM (bare metal or nested virtualization) |

## Design: one container per user session

```
browser ── HTTPS ──▶ gateway (login with the VoipAppz account, routes by session)
                        │
                        ▼
              session container (one per user)
              ├─ Builder core (as today, behind the gateway instead of 127.0.0.1)
              ├─ the project checkout (git clone of the chosen repo and branch)
              ├─ make dev: web app / extension page / Android emulator + stream
              └─ the coding agent (Claude Code over ACP)
```

- **The same binary.** The container runs the existing `voipappz-builder` CLI
  with `--bot-dir <project>`; hosted mode adds a flag for "behind a gateway":
  trust the gateway's authenticated user header instead of the URL secret,
  and build page and stream addresses under the session's public address.
- **Isolation by container**, not by code inside Builder: a user's files,
  processes and ports are their container's. Stopped when idle, removed after
  a time.
- **Projects come from git**, not from a local folder: the user picks a repo
  (their GitHub login, or a deploy key) and a branch; changes leave as a
  branch and a pull request.
- **The local desktop app stays** for those who want everything on their PC.

## Steps (each usable on its own)

| Step | What | Done when |
|---|---|---|
| 1. Remote access to your own Builder | The Builder you run (PC or a server you own) opened from another device's browser: login instead of the URL secret, page and stream proxied under one address, HTTPS through a tunnel | You use your Builder from a tablet with nothing installed there |
| 2. Builder in a container | An image with Builder, the Android tools and the emulator; `docker run` on a KVM server gives the same page | The phone project runs from the image on a server |
| 3. Sessions | A gateway that signs in with the VoipAppz account and starts one container per user; projects cloned from git | Two users work at once without seeing each other |
| 4. The agent per user | Each user's own Claude sign-in (or key) inside their session | A new user chats without touching a terminal |
| 5. Hosting | Where it runs, cost per session, idle shutdown, backups of unpushed work | A week of daily use |

Step 1 is the cheapest and already removes the install on every device but
one. Steps 2 to 5 are the real hosted product.

## Risks

- **Cost:** an Android emulator needs about 4 CPU and 6 GB per session, on
  hardware with KVM. Web and extension sessions are much lighter.
- **Security:** the agent runs commands. On a shared server the container
  boundary is the only wall; it must hold (no shared docker socket, no host
  network, per-session secrets, egress limits).
- **The agent's login:** Claude Code's own sign-in is interactive and per
  person; hosted use may need API keys and their billing.
- **Latency:** the phone stream over the internet is slower than on the PC.
- **Company networks:** an HTTPS-inspecting proxy (as on the owner's PC)
  must allow websockets to the Builder address.

## Open decisions for the owner

1. **Whose server:** your own machine reached remotely (step 1 only), a
   VoipAppz server, or a cloud provider with KVM.
2. **Who uses it:** only you, the team, or customers (this decides how much
   of steps 3 to 5 is needed).
3. **The agent's account:** each user's own Claude login, or one paid API key.
4. **Keep the desktop app** beside the hosted one (recommended), or replace it.
