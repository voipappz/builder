# Assistant dependency audit

Reviewed 2026-10-16 against the local `../voipappz-api` worktree, based on commit
`f993cb09e8246dac38512cf2a43336b24d7b4b05`. The API has unrelated uncommitted
changes, including `lib/models/provider.rb`; findings describe inspected local
code, not proof of the exact deployed source on switch.voipappz.io.

## Result

The current Assistant is not independent of Bot/VML code. It shares a VML HTTP
endpoint and session storage, and constructs a provider adapter under the Bot
namespace. The inspected MCP Assistant path does not invoke Bot orchestration;
the Bot-namespaced dependency is a thin LLM adapter. These are distinct facts.

```text
app AIChat / useAIChat
  POST /api/vmls/generate, mode=mcp
    Endpoints::Vmls
      customer LLM Provider
      vml-index:<customer> / vml-chat:<session>
      Mediators::Mcp::Chat
        Ragents::Agent with its own system prompt and MCP tool registry
        Mediators::Bot::RagentsProvider
          Provider#llm_chat -> per-request RubyLLM context
        Mediators::Mcp::Toolbox -> scoped platform mediators

Same HTTP endpoint with mode absent or different:
  Mediators::Vml::Generate, sharing the session namespace
```

## Evidence

| Location in API | Established behavior | Implication |
|---|---|---|
| `lib/endpoints/vmls.rb:102` | `/generate` is under `/vmls`; selects the customer's enabled LLM provider | Current Assistant transport is part of VML |
| `lib/endpoints/vmls.rb:120` | `mode == mcp` selects an MCP Toolbox; other values do not | A request flag selects the execution branch |
| `lib/endpoints/vmls.rb:125` | Uses `vml-chat:<session>`; new sessions indexed under `vml-index:<customer>` | Assistant and VML histories are not separate domains |
| `lib/endpoints/vmls.rb:176` | MCP branch calls `Mediators::Mcp::Chat` | This is the existing Assistant loop |
| `lib/endpoints/vmls.rb:195` | Other branch calls `Mediators::Vml::Generate` | Missing/wrong mode can select VML generation |
| `lib/mediators/mcp/chat.rb:56` | Creates its own Ragents context, prompt and tool registry | It does not run a Bot definition to answer the chat |
| `lib/mediators/mcp/chat.rb:69` | Defaults to `Mediators::Bot::RagentsProvider.new` | Explicit dependency on Bot-namespaced code |
| `lib/mediators/bot/ragents_provider.rb:3` | RubyLLM provider subclass calling `Provider#llm_chat` | Adapter reuse, not evidence of Bot execution |
| `lib/models/provider.rb:572` | Builds per-request RubyLLM context with provider credentials | Neutral model capability usable without Bot orchestration |
| `lib/endpoints/mcp_tools.rb:20` | Authenticates then dispatches JSON-RPC to Toolbox | MCP endpoint is a tool service, not a chatbot |
| `lib/mediators/mcp/toolbox.rb` | Calls node/cable/device/call/log mediators | No Bot/VML agent call found in this tool implementation |

The endpoint reads a supplied session ID from the VML stream. The new builder
must establish tenant/project/session ownership explicitly before reading,
appending or deleting history; do not copy this storage layout as an isolation
guarantee. This review did not exercise cross-tenant behavior against a server.

## App exposure found during the same review

- TopBar's Assistant action opens `openResourceFinder()`.
- `CommandPalette.jsx` loads `AIChat` for the Assistant dialog.
- `Bots/Bots.jsx` also imports and renders `AIChat`.
- API documentation and global-search helpers contain additional deterministic
  MCP question/answer surfaces.

The requested final boundary is Builder-only chat. Keep the chat UI source in
the app as a shared package that Builder installs (see PLAN.md, "Shared chat UI
package"), remove these app chat access paths, and preserve normal PBX
functions, navigation/search and non-chat API documentation. Builder has no
connection to the Assistant, VML or Bot code, endpoints, storage or libraries,
including Ragents; Builder work does not modify them (PLAN.md decisions, 2026-10-17).

## Required implementation boundary

1. Dedicated `Endpoints::Builder` and builder session/run endpoints, with no
   mode switch to Bot or VML and no fallback to `/api/vmls/generate`.
2. Dedicated builder mediator, prompt, tool registry and per-project session
   storage. Retire the old MCP Assistant branch after caller migration.
3. Builder-owned or neutral model adapter calling the provider abstraction;
   remove the dependency on `Mediators::Bot::RagentsProvider`.
4. Continue using the authenticated MCP Toolbox under caller scope. Builder
   workspace tools are granted separately per project/run.
5. Reuse shared model libraries/provider records only as neutral infrastructure.
   Do not consume Bot definitions, prompts, deployments, tools or histories.

## What must prove independence

The existing MCP chat specs inject `Ragents::Providers::Test`, bypassing the
default Bot provider adapter. They test tool execution/history and filtering,
but cannot prove that the normal provider construction is independent of Bot.
No Bot/VML-independence assertion was found in those focused specs. Specs were
read, not run; no API implementation was changed in this audit.

Add these checks when implementing the dedicated builder path:

- Builder model turn uses its normal adapter while Bot/VML execution entry points
  are configured to fail the test if called; the turn still completes.
- Builder namespaces have no Bot/VML dependency (including autoload/default path).
- Missing/invalid builder parameters are errors, never fallback to VML generation.
- Builder histories cannot load Bot/VML sessions or another tenant/project's ID.
- Builder tools exclude Bot execution and only receive the permitted MCP/workspace
  capability set. Existing bot behavior has separate regression coverage.
- App browser tests confirm no Assistant dialog, shortcut, Bots-page embedding
  or API-docs chat surface remains; Builder chat works through its own endpoints.

## Live endpoint checks and limits

Public MCP initialize/resources succeeded at `https://switch.voipappz.io/tasks/mcp`.
Unauthenticated tools/list at `/api/mcp` returned 401. No authenticated chat/model
run was attempted. Those observations establish connectivity and an auth boundary,
not that the deployed Assistant is already separated or has builder capabilities.
