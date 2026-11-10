# The appz plan

**The target: a customer builds their own application in Builder.** They sign
in, start an appz, tell the agent what their business needs, see it on the
right, talk to its bot, and deploy it to a phone number. They never install a
toolchain, never write React, never see the engine's code. This document is
the plan that gets there, decided on 2026-11-09/10 after reviews by Codex.
It supersedes the Rust `va-agent` direction and the Ionic phone page.

## 1. What a customer's application is

An **appz** is one container image per customer, in VoipAppz's cloud, made of
three layers:

| Layer | Owner | What it holds |
|---|---|---|
| 1. Connectix, the engine | VoipAppz, one version for all | Voice bot pipeline (Feline: STT, Sagents brain, TTS), phone calls over its own SIP user agent (Parrot) and over mod_earshot, bot versions, PocketFlow flows with declared webhooks and a Swagger page, the event pipeline, conversations, the mothership link, metrics, deploy |
| 2. The React catalog | VoipAppz, from the private voipappz/app `packages/*` at a pinned version | Widgets rendered through live_react: chat, conversations, phone (sip.js), dashboard. Props in, callbacks out. Built into the image by Vite, no Node at runtime |
| 3. The customer folder | The customer, through Builder's agent | Elixir only: LiveViews (the screens), handlers, tools, jobs, flows, the bot's prompt and settings, the theme |

Layers 1 and 2 are released together as one image tag. Layer 3 is pulled from
the customer's repository at boot and on every push, in seconds. The customer
never clones Connectix; the engine gains an **extension loader** that compiles
the folder's Elixir at runtime, registers customer skills and flow nodes,
loads the bot, mounts customer LiveViews behind a dispatch plug, and reports
errors back to Builder.

**Data lives in the mothership** (voipappz-api): users, ACLs, calls, numbers,
records, billing. The appz keeps live state only. No database in an appz.

## 2. How a customer screen is built

A screen is a LiveView page. Logic and data are Elixir; React only draws.

| Need | Written in the customer folder | From the catalog |
|---|---|---|
| Page and layout | HEEx with Tailwind | |
| Data and rules | A LiveView: mothership API on mount, NATS or PubSub for changes, the customer's logic in Elixir modules | |
| A rich widget | `<.react name="CallsBoard" calls={@calls} />` | `CallsBoard` in `@voipappz/dashboard-ui` |
| Branding | `config/theme.exs`, exported as CSS variables | Widgets read the same variables |
| A widget that does not exist | A request | Built once in voipappz/app, shipped in the next image, used by every customer |

Examples: a live board of agents and queues (LiveView subscribes to events,
pushes props); custom call handling such as calling back every missed caller
after ten minutes (a handler plus a job, dialing through the appz's own API);
a daily SLA report (an Elixir module computes it, a LiveView shows it).

**Design system of the catalog** (Codex-reviewed): shadcn on Tailwind, adopted
when `dashboard-ui` is built. First restore live_react with the existing MUI
packages (`chat-ui`, `conversations-ui`) and prove a real screen; then port
both and build `dashboard-ui` as one catalog-wide change with screenshot and
keyboard tests. The admin app stays MUI. A lint rule keeps `@mui/*` out of
`packages/*`. A bespoke widget for one customer is a `@voipappz/<customer>-ui`
package bundled only into that customer's image.

## 3. Builder's part

| Builder does | How |
|---|---|
| Sign in with the VoipAppz account | Exists |
| New appz | Clone the small template from GitHub, name it, register it with the mothership |
| Run | Push the folder to the customer's **dev instance** in the cloud, which reloads layer 3; the page pane shows the dev URL (remote-product mode: page and socket URLs and the token from the mothership instead of a local port) |
| Agent | Claude Code over ACP, edits only the customer folder, read-only API tools, the catalog's README as its reference |
| Talk to the bot | The Bot tab's hold-to-talk already speaks mod_earshot's wire protocol; Builder plays the switch against the instance's `/ws/earshot` |
| Deploy | A mothership API call that promotes dev to prod; the fleet runs Kamal. Customers never hold registry, host or SSH credentials |
| Windows | Nothing to install beyond Builder: the runtime is in the cloud. For VoipAppz engineers, Connectix also runs natively (`MODE=native`, mise) or as a Burrito single binary |

Self-serve rules: tenant scope on every mothership call; provider keys
(speech, model) behind the mothership and metered per minute; the agent's
own login either brought by the customer or bundled by VoipAppz, priced
either way.

## 4. Phone calls

| Route | When | How |
|---|---|---|
| Parrot (Connectix's SIP user agent) | First, available today | The instance registers to a VoipAppz switch; outbound: `POST /api/voice/calls` puts the bot on a call (has carried real calls); inbound: a small change lets bot mode answer an inbound INVITE; a number is routed to the extension. The cloud deploy already opens SIP and RTP ports. One bot dialog per user agent: several profiles per appz, or the Earshot route, for volume |
| mod_earshot on the cloud switch | Later, for scale | The switch streams calls to `wss://<appz>/ws/earshot`; needs the node image with mod_earshot (va-crystal branch `node/earshot-base`; rotate the SignalWire token in the parent image before publishing) |

Hebrew: Feline hardcodes Deepgram `nova-2` without a language and Cartesia
`sonic-2`; add language and voice options, or OpenAI speech services, before
promising Hebrew.

## 5. Order of work

Nothing structural before step 2 passes.

| # | Step | Proof |
|---|---|---|
| 1 | Provider fixture test on Connectix with real keys | Green, first-audio latency logged |
| 2 | Real call through Parrot from the cloud instance to a mobile | First audio under 1.5 s after the caller stops, barge-in under 300 ms, five Hebrew turns, recorded |
| 3 | Inbound to the bot, a number routed to it | A call to the number is answered by the bot |
| 4 | Hebrew options in Feline | Native Hebrew understood and spoken clearly |
| 5 | live_react back by hand, Ionic out, Tauri parked, Burrito kept; conversations as the first screen | The screen renders, send and decide round-trip |
| 6 | Extension loader: the customer folder compiled at boot and on push, registries, bot from the folder, errors back to Builder | A folder with one LiveView, one flow, one tool runs on the image |
| 7 | Image build with the catalog pinned to a voipappz/app version | One tag ships engine and UI |
| 8 | Builder: New appz, push to the dev instance, remote-product mode; the template with three samples: a live screen, a custom call flow, a report | Edit, push, see, talk in under five seconds |
| 9 | Provisioning: dev and prod instance per customer, keys and billing through the mothership, Deploy as an API call | Customer one live on a number |
| 10 | Catalog: `dashboard-ui` in shadcn with the port of the chat packages; `phone-ui`; Earshot route; engine as a dependency scoped before customer two | As needed |

## 6. Risks and hedges

| Risk | Hedge |
|---|---|
| The bot has carried real calls over SIP, never over Earshot | Parrot first; Earshot later with the same bar |
| Feline: providers hardcoded, one author, pinned ref | Step 4; vendor Feline like `switchx` if it goes quiet |
| Customer code compiled at runtime can crash the engine | Isolated compile, validated registries, a timeout per skill, middleware order kept, errors returned to Builder |
| One bot dialog per SIP user agent | Several profiles, or Earshot |
| Push-to-see too slow | Only layer 3 reloads; target under five seconds |
| Per-minute provider cost per customer | Keys behind the mothership, minutes metered for billing |
| Two design systems in one repo with agents writing most code | ESLint import boundary in CI |

## 7. Closed

Rust `va-agent` and pipecrab, Lua, a Node server, per-customer React or
shadcn in the customer folder, Ionic, deleting Tauri now (parked instead),
cloning Connectix per customer, merging voipappz/app into Connectix, a new
RTVI voice endpoint, deleting the WebRTC/Parrot/Membrane code, a FreeSWITCH
node on the laptop, CrystalBank.

## 8. Standing rules

Data in the mothership. Customers configure and compose; they do not write
React. Secrets only in the environment. Builder's agent edits only the
customer folder. Confirm every deploy, publish, tag and merge.
