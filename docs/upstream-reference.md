# Upstream reference: what the local server started from

Status: written 2026-10-17 from the upstream desktop app v1.1.0 (Apache-2.0,
see `NOTICE`). Its core and CLI now live in `core/` and `cli/`, renamed to
VoipAppz Builder. This document keeps the mechanisms worth keeping and the
mistakes Builder fixes.

## Shape of the upstream product

| Part | What it was |
|---|---|
| `core` (Rust, axum 0.8, ~9.9k LOC) | Local server on 127.0.0.1:9832: filesystem and shell HTTP endpoints, an HTTP proxy, a download/extract cache, one Phoenix-v2 websocket multiplexing four channels: `watch`, `terminal` (PTY), `acp` (agent process proxy), `mcp` (reverse MCP bridge). Optional HTTPS via rustls. |
| `cli` (clap binary) | Thin wrapper over the server: `--port`, `--debug`, `--allow-remote-access`, `--https-*`, `--allowed-origins`. |
| `src-tauri` (Tauri 2) | Tray-only app, no webview window, no bundled frontend. Runs the server in-process and opens the browser/PWA. Plugins: single-instance, autostart, cli, dialog, opener, updater. |
| Chat UI | Not in the repo. The root page loaded one script from the vendor's website at runtime. Agent loop, model adapter, prompts and tools all lived in that hosted UI or in an external ACP agent. |

Nothing in it was a coding agent. It was a local capability server plus a
process broker for a hosted UI.

## Mechanisms worth re-implementing in Builder

1. **Process-group cancellation.** Every spawned child was placed in its own
   process group (Unix: `SIGKILL` to `-pgid` on drop) or a Windows Job Object
   that terminates the tree on handle close. Streaming shell responses were
   killed when the HTTP client disconnected. Builder's validate/command tool
   needs exactly this; a plain `child.kill()` leaves Vite and npm children alive.
2. **Optimistic-concurrency writes.** `/write` accepted `exclusive` (create only)
   and `expected_sha256` (fail if the file changed). Builder's patch tool uses
   the same idea at revision granularity: expected revision, atomic apply,
   stale write fails.
3. **One multiplexed websocket, Phoenix-channel framing.** `join`/`leave` per
   topic, reply refs, heartbeats. Clean for streaming run events, file-watch
   notices, terminal output and preview logs over a single connection.
4. **Reconnect without replay side effects.** The ACP proxy buffered agent
   notifications per session, pruned them on client ack, and served a custom
   `session/load` so a reconnecting browser caught up without the agent
   re-running anything. It sent `session/cancel` after a disconnect grace
   period and stopped idle agent processes after one minute. Builder's run
   event stream needs the same split: replay by sequence, mutations keyed by
   idempotency key, cancellation on abandoned clients.
5. **Login-shell environment capture.** Vite must be started with the user's
   real PATH (nvm, mise, Homebrew). Upstream spawned the login shell in a PTY
   once and captured its environment. Do this, and strip AppImage variables
   when running on Linux AppImage.
6. **File watcher channel.** `notify`-based, relative paths, debounced. Feeds
   the chat "changes" panel and preview refresh.
7. **Pluggable process starter.** The ACP proxy took a starter function so
   tests could inject a fake process. Keep that pattern for worker tests.
8. **Download cache with checksum.** Tool binaries fetched and extracted under
   a cache dir with NDJSON progress streaming. Reusable for a managed Node
   runtime, provided checksum and signature verification is added.
9. **Release pipeline shape.** tauri-action matrix (macOS arm/x64, Windows x64,
   Linux x64/arm), draft release, macOS P12 keychain import plus notarytool,
   Windows Azure Trusted Signing, updater `latest.json` and signatures from
   `TAURI_SIGNING_PRIVATE_KEY`. Nightly builds gated on recent commits. A
   good template; every secret, identifier and asset name must be ours.

## Mistakes to avoid

- **No authentication on the local server.** The only guard was an Origin
  check that allowed any request with no `Origin` header, so any local process
  could call `/shell` and `/write`. Builder's local server needs a per-launch
  secret that the UI presents, in addition to origin checks.
- **Open proxy.** `ANY /proxy?url=` relayed arbitrary URLs. If Builder needs a
  proxy for the API or Vite, allowlist the configured targets.
- **No workspace root.** Filesystem and shell endpoints accepted any absolute
  path and any cwd. Builder scopes every tool to the canonical project root
  and rejects traversal and symlink escapes.
- **Secrets in logs.** The config `env` map was logged at debug and at info
  level after reload, including values, and injected with `set_var`. Never log
  configuration values; keep secrets in the OS credential store or in
  server-managed grants.
- **Hosted UI dependency.** The product broke without the vendor's website.
  Builder's chat comes from the VoipAppz app.
- **Framework-specific assumptions.** A custom file association, a `web+` protocol
  deep link and the PWA manifest existed without any handling code. Add a file
  association or deep link only with an implementation and tests behind it.

## What upstream did not have and Builder must build

Own chat frontend, agent loop, model adapter, API login and credential store,
MCP client (platform `/api/mcp`, public `/tasks/mcp`, Vite MCP), Vite
supervision and health checks, Admin/User preview windows with separate
sessions, project descriptor parsing and managed workspaces, code search and
patch tools, git integration, local-server auth, installer onboarding, CI.

## Licensing

Upstream is Apache-2.0. `core/` and `cli/` are derived from it, so `LICENSE`
and `NOTICE` stay in the repository and in every installer.
