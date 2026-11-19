import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ChatView, ChatComposer, ToolCallChip, ChatSidebar } from './index';
import { sessions, messages, streamingMessage, errorMessage } from './fixtures';

const baseProps = { inputValue: '', onInputChange: () => {}, onSend: () => {} };

describe('ChatView', () => {
  it('shows the empty state and sends a suggestion', () => {
    const onSend = vi.fn();
    render(<ChatView {...baseProps} onSend={onSend} emptyState={{ title: 'What can I help with?', suggestions: [{ label: 'Try this', prompt: 'do it' }] }} />);
    expect(screen.getByText('What can I help with?')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Try this'));
    expect(onSend).toHaveBeenCalledWith('do it');
  });

  it('renders messages, tool calls with their status, and reasoning', () => {
    render(<ChatView {...baseProps} messages={messages} />);
    expect(screen.getAllByTestId('chat-message')).toHaveLength(2);
    const chips = screen.getAllByTestId('chat-tool-call');
    expect(chips.map(c => c.getAttribute('data-status'))).toEqual(['done', 'running', 'error']);
    expect(screen.getByText('Find the extensions screen')).toBeInTheDocument();
  });

  it('shows the typing indicator for an empty assistant message', () => {
    render(<ChatView {...baseProps} messages={[streamingMessage]} busy />);
    expect(screen.getByTestId('chat-loading')).toBeInTheDocument();
  });

  it('shows an error message without a copy button', () => {
    render(<ChatView {...baseProps} messages={[errorMessage]} />);
    expect(screen.getByText('The run failed.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Copy' })).not.toBeInTheDocument();
  });

  it('hides the sidebar when no sessions array is given', () => {
    render(<ChatView {...baseProps} />);
    expect(screen.queryByTestId('chat-sidebar')).not.toBeInTheDocument();
  });

  it('renders header actions and a footer above the composer', () => {
    render(<ChatView {...baseProps} headerActions={<button>Take over</button>} footer={<div>Approve?</div>} />);
    expect(screen.getByTestId('chat-header-actions')).toHaveTextContent('Take over');
    expect(screen.getByTestId('chat-footer')).toHaveTextContent('Approve?');
  });

  it('renders no header actions or footer by default', () => {
    render(<ChatView {...baseProps} />);
    expect(screen.queryByTestId('chat-header-actions')).not.toBeInTheDocument();
    expect(screen.queryByTestId('chat-footer')).not.toBeInTheDocument();
  });

  it('renders host children such as dialogs', () => {
    render(<ChatView {...baseProps}><div data-testid="host-dialog" /></ChatView>);
    expect(screen.getByTestId('host-dialog')).toBeInTheDocument();
  });
});

describe('ChatSidebar', () => {
  it('selects and deletes sessions through callbacks', () => {
    const onSelect = vi.fn();
    const onDelete = vi.fn();
    render(<ChatSidebar title="Builder" sessions={sessions} selectedSessionId="s1" onSelectSession={onSelect} onDeleteSession={onDelete} />);
    fireEvent.click(screen.getByText('Mobile call history'));
    expect(onSelect).toHaveBeenCalledWith('s2');
    fireEvent.click(screen.getAllByRole('button', { name: 'Delete session' })[0]);
    expect(onDelete).toHaveBeenCalledWith('s1');
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it('shows the status tone and label', () => {
    render(<ChatSidebar title="Builder" sessions={[]} status={{ tone: 'ok', label: 'Connected' }} />);
    expect(screen.getByTestId('chat-status')).toHaveAttribute('data-tone', 'ok');
    expect(screen.getByText('Connected')).toBeInTheDocument();
    expect(screen.getByText('No sessions yet')).toBeInTheDocument();
  });

  it('does not start a new session when disabled', () => {
    const onNew = vi.fn();
    render(<ChatSidebar title="Builder" sessions={[]} onNewSession={onNew} canStartNew={false} />);
    fireEvent.click(screen.getByText('New Chat'));
    expect(onNew).not.toHaveBeenCalled();
  });
});

describe('ChatComposer', () => {
  it('sends on Enter but not on Shift+Enter', () => {
    const onSend = vi.fn();
    render(<ChatComposer value="hello" onChange={() => {}} onSend={onSend} />);
    const input = screen.getByRole('textbox', { name: 'Message' });
    fireEvent.keyDown(input, { key: 'Enter', shiftKey: true });
    expect(onSend).not.toHaveBeenCalled();
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSend).toHaveBeenCalledWith('hello');
  });

  it('does not send blank text', () => {
    const onSend = vi.fn();
    render(<ChatComposer value="   " onChange={() => {}} onSend={onSend} />);
    expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'Message' }), { key: 'Enter' });
    expect(onSend).not.toHaveBeenCalled();
  });

  it('shows a stop button while busy', () => {
    const onStop = vi.fn();
    render(<ChatComposer value="" onChange={() => {}} onSend={() => {}} onStop={onStop} busy />);
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
    expect(onStop).toHaveBeenCalled();
  });

  it('reports typed text as a plain string', () => {
    const onChange = vi.fn();
    render(<ChatComposer value="" onChange={onChange} onSend={() => {}} />);
    fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), { target: { value: 'abc' } });
    expect(onChange).toHaveBeenCalledWith('abc');
  });
});

describe('ToolCallChip', () => {
  it('defaults to running when no status is given', () => {
    render(<ToolCallChip toolCall={{ id: 'x', name: 'search' }} />);
    expect(screen.getByTestId('chat-tool-call')).toHaveAttribute('data-status', 'running');
  });
});
