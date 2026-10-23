# Builder contracts v1

The versioned data contracts that the Builder desktop shell, its local server
and the VoipAppz API share. The JSON Schemas in `v1/` are the source of truth.

| Contract | What it describes |
|---|---|
| `project-descriptor` | The local file kept with a customer project. It never holds a secret; `credentialRef` names an OS credential store entry. The optional `chatClient.url` names where the stub page loads the chat client from. |
| `connection-state` | What the shell shows about its link to the API, project and worker. Each failure is its own state. |
| `project` | A Builder project as the API returns it. No local paths. |
| `checkpoint` | A saved revision whose passed validation belongs to that exact revision. |
| `error` | One error shape for API responses and tool failures. |

Conversations are not defined here. The chat client drives the agent over ACP
(Agent Client Protocol), so sessions, prompt turns, streamed text, tool calls
and session reload use ACP's own messages. Tools are MCP tools. The earlier
`session`, `run` and `event` contracts were removed for that reason.

## Rules the schemas cannot express

`validate()` adds this check:

- A checkpoint's validation revision equals its revision.

## Shared fixtures

`fixtures/<contract>/valid` and `fixtures/<contract>/invalid` hold example
documents. Every implementation must accept every valid fixture and reject
every invalid one at the path recorded in `expected-errors.json`. The API
vendors `v1/` and `fixtures/` and runs the same cases in Ruby.

## Use

```js
import { validate } from '@voipappz/builder-contracts';

validate('project-descriptor', descriptor); // { valid, errors: [{ path, message }] }
```

## Changing a contract

Additive optional fields stay in v1. Anything that makes a previously valid
document invalid, or changes a meaning, goes in `v2/`. Update the fixtures in
the same change, then copy `v1/` and `fixtures/` into the API.

```text
npm test
```
