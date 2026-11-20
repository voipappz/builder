// Maps ACP `session/update` notifications onto the message shape
// `@voipappz/chat-ui` draws: { id, role, content, createdAt, error,
// toolCalls: [{ id, name, args, result, status }], reasoning: [{ title }] }.
// Pure functions: each returns a new messages array.

const TOOL_STATUS = { pending: 'running', in_progress: 'running', completed: 'done', failed: 'error' };
const PLAN_MARK = { pending: '○', in_progress: '◐', completed: '●' };

let counter = 0;
const newId = (prefix) => `${prefix}-${Date.now()}-${++counter}`;

const textOf = (block) => (block?.type === 'text' ? block.text : '');

function toolResult(update) {
  const parts = (update.content || []).map((item) => {
    if (item.type === 'content') return textOf(item.content);
    if (item.type === 'diff') return `Edited ${item.path}`;
    if (item.type === 'terminal') return `Terminal ${item.terminalId}`;
    return '';
  }).filter(Boolean);
  if (parts.length) return parts.join('\n');
  if (update.rawOutput !== undefined) {
    return typeof update.rawOutput === 'string' ? update.rawOutput : JSON.stringify(update.rawOutput);
  }
  return undefined;
}

function withReasoning(message) {
  const steps = (message.plan || []).map((entry) => ({ title: `${PLAN_MARK[entry.status] || '○'} ${entry.content}` }));
  if (message.thought) steps.push({ title: message.thought });
  return { ...message, reasoning: steps };
}

// The assistant message of the current turn, created on first use.
function updateAssistant(messages, now, change) {
  const last = messages[messages.length - 1];
  if (last?.role === 'assistant' && !last.done) {
    return [...messages.slice(0, -1), change(last)];
  }
  const fresh = { id: newId('assistant'), role: 'assistant', content: '', createdAt: now, toolCalls: [], reasoning: [] };
  return [...messages, change(fresh)];
}

export function addUserMessage(messages, text, now = new Date().toISOString()) {
  return [...messages, { id: newId('user'), role: 'user', content: text, createdAt: now }];
}

export function applyUpdate(messages, update, now = new Date().toISOString()) {
  switch (update?.sessionUpdate) {
    case 'user_message_chunk': {
      const last = messages[messages.length - 1];
      if (last?.role === 'user') {
        return [...messages.slice(0, -1), { ...last, content: last.content + textOf(update.content) }];
      }
      return addUserMessage(messages, textOf(update.content), now);
    }
    case 'agent_message_chunk':
      return updateAssistant(messages, now, (m) => ({ ...m, content: m.content + textOf(update.content) }));
    case 'agent_thought_chunk':
      return updateAssistant(messages, now, (m) => withReasoning({ ...m, thought: (m.thought || '') + textOf(update.content) }));
    case 'plan':
      return updateAssistant(messages, now, (m) => withReasoning({ ...m, plan: update.entries || [] }));
    case 'tool_call':
      return updateAssistant(messages, now, (m) => ({
        ...m,
        toolCalls: [...m.toolCalls, {
          id: update.toolCallId,
          name: update.title || update.kind || 'tool',
          args: update.rawInput,
          result: toolResult(update),
          status: TOOL_STATUS[update.status] || 'running'
        }]
      }));
    case 'tool_call_update':
      return updateAssistant(messages, now, (m) => ({
        ...m,
        toolCalls: m.toolCalls.map((call) => (call.id !== update.toolCallId ? call : {
          ...call,
          name: update.title || call.name,
          args: update.rawInput ?? call.args,
          result: toolResult(update) ?? call.result,
          status: update.status ? (TOOL_STATUS[update.status] || call.status) : call.status
        }))
      }));
    default:
      return messages;
  }
}

// Close the turn: later chunks start a new assistant message.
export function finishTurn(messages, { stopReason, error } = {}) {
  const last = messages[messages.length - 1];
  if (last?.role !== 'assistant' || last.done) {
    if (!error) return messages;
    return [...messages, { id: newId('assistant'), role: 'assistant', content: '', error, done: true, createdAt: new Date().toISOString() }];
  }
  const note = stopReason === 'cancelled' ? 'Stopped.' : undefined;
  return [...messages.slice(0, -1), {
    ...last,
    done: true,
    error: error || last.error,
    content: last.content || note || last.content
  }];
}
