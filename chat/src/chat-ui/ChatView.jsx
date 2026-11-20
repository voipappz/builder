import { useEffect, useRef } from 'react';
import { Avatar, Box, CircularProgress, Typography } from '@mui/material';
import SmartToyIcon from '@mui/icons-material/SmartToy';
import ChatSidebar from './ChatSidebar';
import ChatMessage from './ChatMessage';
import ChatComposer from './ChatComposer';
import ChatEmptyState from './ChatEmptyState';

/**
 * Full chat layout: optional session sidebar, header, messages, composer.
 *
 * Fully controlled. The host owns messages, sessions, input text and busy
 * state, and receives every user action through a callback. This component
 * does no fetching, streaming, storage or auth.
 *
 * headerActions: nodes at the end of the header (for example a take-over
 * toggle). footer: nodes between the messages and the composer (for example
 * an approval prompt the user must answer before typing).
 */
const ChatView = ({
  title = 'Assistant',
  subtitle,
  icon,
  status,
  messages = [],
  sessions,
  selectedSessionId,
  isLoadingSessions = false,
  isRestoring = false,
  busy = false,
  inputValue,
  onInputChange,
  onSend,
  onStop,
  onSelectSession,
  onDeleteSession,
  onNewSession,
  sidebarCollapsed = false,
  onToggleSidebar,
  emptyState = {},
  composer = {},
  labels = {},
  headerActions,
  footer,
  children
}) => {
  const endRef = useRef(null);
  const inputRef = useRef(null);
  const showSidebar = Array.isArray(sessions);

  useEffect(() => {
    endRef.current?.scrollIntoView?.({ behavior: 'smooth' });
  }, [messages]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <Box data-testid="chat-view" sx={{ display: 'flex', height: '100%', backgroundColor: 'var(--theme-bg-primary)' }}>
      {showSidebar && (
        <ChatSidebar
          title={title}
          status={status}
          sessions={sessions}
          selectedSessionId={selectedSessionId}
          isLoadingSessions={isLoadingSessions}
          onSelectSession={onSelectSession}
          onDeleteSession={onDeleteSession}
          onNewSession={onNewSession}
          canStartNew={messages.length > 0}
          collapsed={sidebarCollapsed}
          onToggle={onToggleSidebar}
          labels={labels}
        />
      )}

      <Box sx={{ flex: 1, display: 'flex', flexDirection: 'column', position: 'relative', minWidth: 0 }}>
        <Box sx={{
          p: 2,
          borderBottom: '1px solid var(--theme-border)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          backgroundColor: 'var(--theme-bg-secondary)'
        }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            <Avatar sx={{ bgcolor: 'var(--accent-primary, #65758E)', width: 36, height: 36 }}>
              {icon || <SmartToyIcon sx={{ fontSize: 20 }} />}
            </Avatar>
            <Box>
              <Typography sx={{ color: 'var(--theme-text-primary)', fontWeight: 600, fontSize: '0.95rem' }}>{title}</Typography>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                {subtitle && (
                  <Typography sx={{ color: 'var(--theme-text-tertiary)', fontSize: '0.75rem' }}>{subtitle}</Typography>
                )}
                {busy && <CircularProgress size={12} sx={{ color: 'var(--accent-primary, #65758E)' }} />}
              </Box>
            </Box>
          </Box>
          {headerActions && (
            <Box data-testid="chat-header-actions" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              {headerActions}
            </Box>
          )}
        </Box>

        <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
          {isRestoring && messages.length === 0 ? (
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', gap: 1.5 }}>
              <CircularProgress size={20} sx={{ color: 'var(--accent-primary, #65758E)' }} />
              <Typography sx={{ color: 'var(--theme-text-tertiary)', fontSize: '0.85rem' }}>
                {labels.restoring || 'Loading your last conversation…'}
              </Typography>
            </Box>
          ) : messages.length === 0 ? (
            <ChatEmptyState {...emptyState} onSuggestion={emptyState.onSuggestion || onSend} />
          ) : (
            <Box sx={{ maxWidth: 800, mx: 'auto', display: 'flex', flexDirection: 'column', gap: 1.5 }}>
              {messages.map((message) => <ChatMessage key={message.id} message={message} labels={labels} />)}
              <div ref={endRef} />
            </Box>
          )}
        </Box>

        {footer && <Box data-testid="chat-footer">{footer}</Box>}

        <ChatComposer
          ref={inputRef}
          value={inputValue}
          onChange={onInputChange}
          onSend={onSend}
          onStop={onStop}
          busy={busy}
          labels={labels}
          {...composer}
        />
      </Box>
      {children}
    </Box>
  );
};

export default ChatView;
