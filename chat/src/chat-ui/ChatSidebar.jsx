import { memo } from 'react';
import { Box, Divider, IconButton, Skeleton, Typography } from '@mui/material';
import AddIcon from '@mui/icons-material/Add';
import ChatBubbleOutlineIcon from '@mui/icons-material/ChatBubbleOutline';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import DeleteOutlineIcon from '@mui/icons-material/DeleteOutline';
import SmartToyIcon from '@mui/icons-material/SmartToy';

const STATUS_COLOR = {
  ok: 'var(--color-success)',
  error: 'var(--color-danger)',
  warn: 'var(--color-warning, #f59e0b)',
  checking: 'var(--color-neutral)'
};

const SessionItem = memo(({ session, isSelected, onSelect, onDelete, deleteLabel }) => (
  <Box
    role="button"
    tabIndex={0}
    aria-current={isSelected ? 'true' : undefined}
    data-testid="chat-session"
    onClick={() => onSelect?.(session.id)}
    onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect?.(session.id); } }}
    sx={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      p: 1.5,
      borderRadius: '8px',
      cursor: 'pointer',
      backgroundColor: isSelected ? 'var(--theme-bg-secondary)' : 'transparent',
      '&:hover': { backgroundColor: 'var(--theme-hover)' },
      '&:hover .va-chat-session-delete, &:focus-within .va-chat-session-delete': { opacity: 1 },
      transition: 'background-color 0.2s'
    }}
  >
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, overflow: 'hidden' }}>
      <ChatBubbleOutlineIcon sx={{ fontSize: 16, color: 'var(--theme-text-tertiary)', flexShrink: 0 }} />
      <Typography
        sx={{
          fontSize: '0.8rem',
          color: isSelected ? 'var(--theme-text-primary)' : 'var(--theme-text-secondary)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap'
        }}
      >
        {session.title}
      </Typography>
    </Box>
    {onDelete && (
      <IconButton
        size="small"
        className="va-chat-session-delete"
        aria-label={deleteLabel}
        onClick={(e) => { e.stopPropagation(); onDelete(session.id); }}
        sx={{ opacity: 0, color: 'var(--theme-text-tertiary)', '&:hover': { color: '#ef4444' } }}
      >
        <DeleteOutlineIcon sx={{ fontSize: 16 }} />
      </IconButton>
    )}
  </Box>
));
SessionItem.displayName = 'SessionItem';

/**
 * Collapsible session list.
 *
 * sessions: [{ id, title }]
 * status:   { tone: 'ok' | 'warn' | 'error' | 'checking', label } (optional)
 */
const ChatSidebar = ({
  title,
  icon,
  status,
  sessions = [],
  selectedSessionId,
  isLoadingSessions = false,
  onSelectSession,
  onDeleteSession,
  onNewSession,
  canStartNew = true,
  collapsed = false,
  onToggle,
  labels = {}
}) => (
  <Box
    data-testid="chat-sidebar"
    sx={{
      width: collapsed ? 40 : 256,
      height: '100%',
      backgroundColor: 'var(--theme-bg-secondary)',
      borderRight: '1px solid var(--theme-border)',
      display: 'flex',
      flexDirection: 'column',
      transition: 'width 0.3s ease',
      overflow: 'hidden',
      flexShrink: 0,
      position: 'relative'
    }}
  >
    {onToggle && (
      <IconButton
        onClick={onToggle}
        aria-label={collapsed ? (labels.expandSidebar || 'Show sessions') : (labels.collapseSidebar || 'Hide sessions')}
        sx={{ position: 'absolute', right: collapsed ? 0 : 8, top: 8, color: 'var(--theme-text-secondary)', zIndex: 10 }}
      >
        {collapsed ? <ChevronRightIcon /> : <ChevronLeftIcon />}
      </IconButton>
    )}

    {!collapsed && (
      <Box sx={{ p: 2, display: 'flex', flexDirection: 'column', gap: 2, height: '100%' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          {icon || <SmartToyIcon sx={{ color: 'var(--accent-primary, #65758E)', fontSize: 20 }} />}
          <Typography sx={{ color: 'var(--theme-text-primary)', fontSize: '0.75rem', fontWeight: 600, textTransform: 'uppercase' }}>
            {title}
          </Typography>
        </Box>

        {onNewSession && (
          <Box
            role="button"
            tabIndex={canStartNew ? 0 : -1}
            aria-disabled={!canStartNew}
            onClick={canStartNew ? onNewSession : undefined}
            onKeyDown={(e) => { if (canStartNew && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); onNewSession(); } }}
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 1,
              p: 1.5,
              backgroundColor: 'var(--accent-primary, #65758E)',
              color: '#fff',
              borderRadius: '12px',
              cursor: canStartNew ? 'pointer' : 'not-allowed',
              opacity: canStartNew ? 1 : 0.5,
              '&:hover': { opacity: canStartNew ? 0.9 : 0.5 }
            }}
          >
            <AddIcon sx={{ fontSize: 16 }} />
            <Typography sx={{ fontSize: '0.75rem', fontWeight: 600, textTransform: 'uppercase' }}>
              {labels.newSession || 'New Chat'}
            </Typography>
          </Box>
        )}

        {status && (
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }} data-testid="chat-status" data-tone={status.tone}>
            <Box sx={{ width: 8, height: 8, borderRadius: '50%', backgroundColor: STATUS_COLOR[status.tone] || STATUS_COLOR.checking }} />
            <Typography sx={{ fontSize: '0.65rem', color: 'var(--theme-text-tertiary)' }}>{status.label}</Typography>
          </Box>
        )}

        <Divider sx={{ borderColor: 'var(--theme-border)' }} />

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1, flex: 1, overflow: 'hidden' }}>
          <Typography sx={{ color: 'var(--theme-text-primary)', fontSize: '0.7rem', fontWeight: 600, textTransform: 'uppercase' }}>
            {labels.sessions || 'Sessions'}
          </Typography>
          <Box sx={{ flex: 1, overflow: 'auto' }}>
            {isLoadingSessions ? (
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                {[1, 2, 3].map(i => (
                  <Skeleton key={i} variant="rounded" height={40} sx={{ backgroundColor: 'var(--theme-bg-secondary)' }} />
                ))}
              </Box>
            ) : sessions.length === 0 ? (
              <Box sx={{ textAlign: 'center', py: 4 }}>
                <ChatBubbleOutlineIcon sx={{ fontSize: 32, color: 'var(--theme-border)', mb: 1 }} />
                <Typography sx={{ fontSize: '0.75rem', color: 'var(--theme-text-tertiary)' }}>
                  {labels.noSessions || 'No sessions yet'}
                </Typography>
              </Box>
            ) : (
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
                {sessions.map((session) => (
                  <SessionItem
                    key={session.id}
                    session={session}
                    isSelected={selectedSessionId === session.id}
                    onSelect={onSelectSession}
                    onDelete={onDeleteSession}
                    deleteLabel={labels.deleteSession || 'Delete session'}
                  />
                ))}
              </Box>
            )}
          </Box>
        </Box>
      </Box>
    )}
  </Box>
);

export default ChatSidebar;
