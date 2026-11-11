# The demo: your calls, in your own app, built with Builder

What a prospect sees in ten minutes, and what has to be true beforehand.

## Before

- **Builder** installed: the latest release's Windows exe, .deb or AppImage
  (github.com/voipappz/builder/releases). On Windows, WSL with `mise`
  installed, because the appz runs there.
- **The appz folder**: a Connectix checkout (`connectix-io/connectix`,
  `main`). Its first `make setup` takes four to five minutes (Elixir, Node,
  the page's assets) and runs once.
- **A VoipAppz account** that has calls: the dashboard reads them from
  switch.voipappz.io as that user. Builder trades the sign-in for the token
  itself; nothing is pasted into `.env`.

## The ten minutes

1. **Open Builder, sign in** with the VoipAppz account, choose the appz
   folder. Builder runs the folder, the left pane is the chat with the
   coding agent, the right pane is the appz's first page.
2. **The Calls dashboard** (`/app/dashboard`) shows today's calls: totals,
   answer rate, the hour-by-hour chart, the calls up right now, the newest
   calls. Switch to seven or thirty days. That is a LiveView in the
   customer's Elixir app rendering a React widget from the VoipAppz catalog,
   with the data coming from the mothership.
3. **Ask the agent for a change**, for example: "Add a tile with the busiest
   hour of the day." It edits the LiveView (the logic is Elixir, in the
   appz) and the page reloads in the right pane. Logic in the customer's
   app; screens from the catalog; data in the mothership.
4. **Status tab**: the appz's log, restart, the tests, the deploy actions.

## When something is off

| You see | Why | Do |
|---|---|---|
| "Not connected to VoipAppz" | no token reached the appz | sign in in Builder (desktop app); or put `MOTHERSHIP_API_URL` and `MOTHERSHIP_API_TOKEN` in the folder's `.env` |
| "VoipAppz refused the token" | the sign-in session ended (about eight hours) and could not be refreshed, or the account may not read calls | quit Builder and sign in again; a restart from the Status tab reuses the old tokens |
| The pane is blank | the appz still starting, or its `.env` sets the operator Basic Auth pair on an old Connectix | wait for "ready" in Status; update Connectix (`main` lets loopback through in dev) |
| "Your account asks for a one-time code" | OTP is on for the account | use an account without OTP for the demo, or set the token in `.env` |

## What the token is, and who can reach it

Builder trades the sign-in for the user's own VoipAppz session tokens and
passes them to one command only, the appz's `make dev`. Setup, the action
buttons and the coding agent's process do not get them, on Linux or through
WSL. That is a separation of environments, not a wall: the agent edits the
appz's code, and code in that process can read its environment. Until the
mothership issues a read-only, short-lived token for an appz, demo with an
account whose access you are content to lend to the code being built. A
remote appz (`url` in builder.toml) gets no token from Builder at all; its
instance carries its own.
