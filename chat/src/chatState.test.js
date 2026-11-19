import { describe, expect, it } from 'vitest';
import { addUserMessage, applyUpdate, finishTurn } from './chatState';

const text = (t) => ({ type: 'text', text: t });

describe('Builder chat state', () => {
  it('appends agent text to one assistant message per turn', () => {
    let messages = addUserMessage([], 'hi');
    messages = applyUpdate(messages, { sessionUpdate: 'agent_message_chunk', content: text('Hel') });
    messages = applyUpdate(messages, { sessionUpdate: 'agent_message_chunk', content: text('lo') });
    expect(messages).toHaveLength(2);
    expect(messages[1]).toMatchObject({ role: 'assistant', content: 'Hello' });
  });

  it('starts a new assistant message after the turn finished', () => {
    let messages = applyUpdate([], { sessionUpdate: 'agent_message_chunk', content: text('one') });
    messages = finishTurn(messages, { stopReason: 'end_turn' });
    messages = applyUpdate(messages, { sessionUpdate: 'agent_message_chunk', content: text('two') });
    expect(messages.map((m) => m.content)).toEqual(['one', 'two']);
  });

  it('tracks a tool call from start to result', () => {
    let messages = applyUpdate([], {
      sessionUpdate: 'tool_call', toolCallId: 't1', title: 'Read src/App.jsx', status: 'pending', rawInput: { path: 'src/App.jsx' }
    });
    expect(messages[0].toolCalls[0]).toMatchObject({ id: 't1', name: 'Read src/App.jsx', status: 'running' });
    messages = applyUpdate(messages, {
      sessionUpdate: 'tool_call_update', toolCallId: 't1', status: 'completed',
      content: [{ type: 'content', content: text('file body') }]
    });
    expect(messages[0].toolCalls[0]).toMatchObject({ status: 'done', result: 'file body', args: { path: 'src/App.jsx' } });
  });

  it('marks failed tools and summarizes diffs', () => {
    let messages = applyUpdate([], { sessionUpdate: 'tool_call', toolCallId: 't1', title: 'Edit', status: 'in_progress' });
    messages = applyUpdate(messages, {
      sessionUpdate: 'tool_call_update', toolCallId: 't1', status: 'failed',
      content: [{ type: 'diff', path: 'src/a.js', oldText: 'a', newText: 'b' }]
    });
    expect(messages[0].toolCalls[0]).toMatchObject({ status: 'error', result: 'Edited src/a.js' });
  });

  it('shows plan entries and thoughts as reasoning steps', () => {
    let messages = applyUpdate([], {
      sessionUpdate: 'plan', entries: [{ content: 'Find it', status: 'completed' }, { content: 'Fix it', status: 'pending' }]
    });
    messages = applyUpdate(messages, { sessionUpdate: 'agent_thought_chunk', content: text('thinking') });
    expect(messages[0].reasoning.map((s) => s.title)).toEqual(['● Find it', '○ Fix it', 'thinking']);
  });

  it('replays user messages on session load', () => {
    const messages = applyUpdate([], { sessionUpdate: 'user_message_chunk', content: text('earlier question') });
    expect(messages[0]).toMatchObject({ role: 'user', content: 'earlier question' });
  });

  it('records an error turn and ignores unknown updates', () => {
    let messages = applyUpdate(addUserMessage([], 'go'), { sessionUpdate: 'available_commands_update' });
    expect(messages).toHaveLength(1);
    messages = finishTurn(messages, { error: 'The agent stopped' });
    expect(messages[1]).toMatchObject({ role: 'assistant', error: 'The agent stopped' });
  });
});
