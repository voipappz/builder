# Vision: what Builder is for

Written 2026-11-10 from a Codex vision review of `appz-plan.md` and
`builder-plan.md`. The answers to its challenges are recommendations for the
owner to confirm, not decisions yet.

## Positioning

Builder is the fastest way for a telecom operator, a contact center or a
service business to turn its live communications operation into a tailored,
deployed application. It will be compared with Lovable and Bolt (build by
chat), Retool (internal tools), Zapier and Twilio Studio (workflows) and Vapi
and Bland (voice agents). What is different: VoipAppz owns the telephony and
the operational data, generates governed Elixir applications, and runs every
customer on one maintained engine instead of stitching SaaS APIs together.

## Three horizons

| Horizon | Promise | What customers build |
|---|---|---|
| H1, 3 months | "Describe one communications operation, see it working with your real data, attach a number, deploy." Outcomes, not an open-ended IDE | 1. AI receptionist and overflow desk. 2. Missed-call recovery. 3. Queue supervisor cockpit. 4. Outbound campaign concierge. 5. Conversation QA and follow-up |
| H2, 6–9 months | The extension loader, a stable catalog, provisioning, authentication, metering, promote and rollback turn demos into a repeatable product | Branded portals, specialized workflows, reports, webhooks, bot profiles, scheduled jobs, CRM handoffs, approval queues. "Call every abandoned caller within ten minutes unless an agent already responded" becomes a versioned flow, a dashboard, an audit trail and a deployed service |
| H3 | An application ecosystem for communications businesses | A marketplace of reviewed flows, skills, integrations and widgets; vertical templates (clinics, property, automotive, collections, field service); operational agents that watch queues and act with approval; a multi-tenant engine |

The five H1 applications reuse the same primitives: numbers, routing, calls,
queues, agents, campaigns, conversations, reports. One coherent catalog,
credible demos, not five mini-products.

## What is defensible, and the traps

Defensible: the telecom system of record (identity, ACLs, numbers, routing,
calls, queues, recordings, conversations, billing); a real-time engine whose
flows have explicit triggers, typed inputs, whitelisted nodes, observable
outcomes, isolation and back-pressure; a curated UI catalog bound to that
data model; the path from conversational intent to a callable number and a
deployed application; vertical templates and evaluated flows accumulated
across deployments. Generated code itself will commoditize.

Traps: becoming "Retool with a phone widget" (arbitrary page building,
arbitrary dependencies, a widget per request, unreviewed runtime code);
voice economics (a good agent with poor turn latency or unbounded STT, LLM
and TTS cost is not a product: meter every stage, show cost per completed
outcome, use deterministic flow paths where an LLM adds nothing).

## Monetisation levers

Per appz and environment (base subscription: dev, prod, monitoring, kept
versions); communications usage (minutes, numbers, recordings, transcription,
messaging at pass-through plus margin); seats (supervisors, agents, Builder
collaborators); deployed automations (premium workflows, profiles,
concurrency, SLA); agent usage (an included build allowance, then metered);
marketplace share and paid templates; enterprise (regional hosting, audit
retention, private integrations). Do not charge per deploy: it discourages
iteration.

## Three plan decisions Codex challenges, and the recommended answer

| Challenge | Alternative proposed | Recommendation (to confirm) |
|---|---|---|
| Customer-authored Elixir compiled into the running engine contradicts the whitelisted-node safety model | Default to declarative flows, profiles, tools and page composition; custom Elixir behind a reviewed extension boundary with a capability manifest, tests, resource limits and a separate failure domain | Adopt as the shape of the extension loader: flows and composition first; customer Elixir loaded in isolation with limits and a manifest |
| One container and a dev/prod pair per customer as the permanent unit is costly and slow to scale | Keep the appz abstraction; design toward pooled multi-tenant engine workers with isolated execution for custom extensions | Adopt as the H3 direction; per-customer containers for the first customers |
| A broad new shadcn catalog before validating applications risks becoming the critical path | Productize the smallest existing components the five H1 applications need, with stable data and action contracts; restyle only when reuse justifies it | Adopt: the catalog grows by the H1 applications' needs; the shadcn switch waits for `dashboard-ui` (see builder-plan.md) |

Also recommended: git push is plumbing, not the customer's mental model. Builder
saves an immutable appz version and promotes it; repository mechanics stay
below the surface.

## The ten-minute demo: missed-call recovery for a plumbing company

1. Sign in, New appz, "Missed-call recovery".
2. Say: "If nobody answers within 20 seconds, call the customer back, ask
   whether it is an emergency, collect postcode and issue, transfer
   emergencies to the on-call agent. Show supervisors today's recoveries."
3. Builder produces a flow, a bot profile and a supervisor page, explains
   its choices, and asks before connecting a number.
4. Call the number and let it ring out.
5. The prospect's mobile rings; the bot collects the details and transfers
   or schedules a follow-up.
6. The right pane shows the original call, the transcript, the
   classification, the collected fields, the outcome and the responsible
   agent, live.
7. Ask: "Never call back twice, and require approval after 10 p.m." Builder
   changes the flow; the prospect sees the new branch, not a prompt.
8. Run a scenario test, deploy, watch health, show the version and one-click
   rollback.

One story that exercises telephony, data, agent construction, UI, governance
and deployment.
