# Builder chat client

The chat screen of [VoipAppz Builder](https://github.com/voipappz/builder).
Builder's local server serves a small page on `http://127.0.0.1:9832` that
loads this chat: built into `../client/` (`npm run build`, embedded by
`core/build.rs`), or live from `npm run dev` with
`VOIPAPPZ_BUILDER_CLIENT_DEV=http://localhost:5174`. Moved here from voipappz/app on 2026-11-19;
`src/chat-ui/` is a copy of that repo's `packages/chat-ui` (MIT).

| File | Job |
|---|---|
| `main.jsx` | Entry. Reads the page's `voipappz-builder:info` meta tag and renders `BuilderApp` with the app theme |
| `BuilderApp.jsx` | Layout: `ChatView` from `src/chat-ui`, agent mode picker, permission bar, Admin and User previews |
| `useBuilderChat.js` | One chat with the agent: session, messages, busy state, permission requests, modes |
| `acpConnection.js` | ACP (Agent Client Protocol) as raw JSON-RPC over the server's `acp:` channel |
| `phoenixSocket.js` | Minimal Phoenix v2 websocket client for that channel |
| `BotPanel.jsx` | The Bot tab, when Builder runs a voice bot: status, restart, log, hold to talk. For a product (Connectix, `kind: "product"`) the tab is called Status and has no talk button; the product's own page (`pageUrl`) is a tab of its own, first. The folder's actions (`actions`, e.g. tests or a deploy) are buttons; one marked `confirm` asks first |
| `captureWorklet.js` | Audio-thread microphone capture in 20 ms packets, emitted as its own file (Builder's page policy allows no `blob:` scripts) |
| `botCall.js` | One call to the bot through Builder's bridge, in Earshot's native framing (PCM16 16 kHz, 20 ms packets, `speech_started`/`speech_stopped`, `clear`) |
| `chatState.js` | Maps ACP `session/update` notifications onto the chat-ui message shape (tested in `chatState.test.js`) |

## Rules

- The client never names a command, folder or credential. Builder's server
  picks the agent, its folder and its MCP servers, and enforces that the agent
  writes only in Builder's worktree of this app and uses the API read-only.
- New sessions switch to the agent's "Accept edits" mode: edits inside the
  worktree go through, commands still ask through the permission bar.
- The previews run on `admin.localhost` and `user.localhost`, so the two
  logins (kept in `localStorage` per origin) stay apart.

## Working on it

Node 22 (`.nvmrc`). `npm ci`, then `npm test` (Vitest) and `npm run build`
(writes `../client/`). Live: `npm run dev` (port 5174), then start Builder
with `VOIPAPPZ_BUILDER_CLIENT_DEV=http://localhost:5174`.
How the whole system fits together: [docs/how-it-works.md](../docs/how-it-works.md).
